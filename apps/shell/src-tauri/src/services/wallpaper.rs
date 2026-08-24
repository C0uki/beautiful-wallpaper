//! Applying wallpapers, generating thumbnails, and fetching online ones.
//!
//! The equivalent of end4-pC's `services/Wallpapers.qml` plus
//! `services/OnlineWallpapers.qml`, minus the `curl`, ImageMagick and Python
//! subprocesses.

use std::io::Write;
use std::path::{Path, PathBuf};

use bw_core::wallpaper::online::{self, Provider, Query, WallpaperPage};
use image::imageops::FilterType;

use crate::state::{AppState, WallpaperState};

/// Applies a wallpaper: tells Windows about it, records it, and re-themes.
pub fn apply(state: &AppState, path: &str) -> Result<(), String> {
    let config = state.config();

    let blanked = bw_core::wallpaper::is_work_unsafe(Path::new(path), &config.work_safety.keywords)
        && config.work_safety.blank_wallpaper;

    // Even when blanked, record the real path: the user chose it, and unticking
    // work-safety should bring it straight back.
    state
        .set_config_value(
            "background.wallpaperPath",
            serde_json::Value::String(path.to_owned()),
        )
        .map_err(|error| error.to_string())?;

    state.record_wallpaper(WallpaperState {
        monitor: String::new(),
        path: path.to_owned(),
        blanked,
    });

    set_desktop_wallpaper(path)?;
    Ok(())
}

#[cfg(windows)]
fn set_desktop_wallpaper(path: &str) -> Result<(), String> {
    crate::platform::wallpaper::set(path, None, crate::platform::wallpaper::Fit::Fill)
        .map_err(|error| format!("could not set the desktop wallpaper: {error}"))
}

#[cfg(not(windows))]
fn set_desktop_wallpaper(_path: &str) -> Result<(), String> {
    Ok(())
}

/// Lists a directory for the wallpaper browser.
pub fn list(
    state: &AppState,
    path: Option<&str>,
) -> Result<Vec<bw_core::wallpaper::Entry>, String> {
    let config = state.config();
    let directory = match path {
        Some(path) if !path.is_empty() => PathBuf::from(path),
        _ if !config.wallpaper_selector.user_path.is_empty() => {
            PathBuf::from(&config.wallpaper_selector.user_path)
        }
        _ => bw_core::paths::default_wallpaper_dir(),
    };

    bw_core::wallpaper::list_directory(&directory, &config.wallpaper_selector.extensions)
        .map_err(|error| format!("could not list {}: {error}", directory.display()))
}

/// Picks a different wallpaper from the current folder.
pub fn random(state: &AppState) -> Result<String, String> {
    let current = state.config().background.wallpaper_path;
    let folder = Path::new(&current)
        .parent()
        .map(|parent| parent.to_string_lossy().into_owned());

    let entries = list(state, folder.as_deref())?;
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos() as u64)
        .unwrap_or(1);

    let picked = bw_core::wallpaper::next_random(&entries, Some(current.as_str()), seed)
        .ok_or_else(|| "there is no other wallpaper in this folder".to_owned())?;

    let path = picked.to_string_lossy().into_owned();
    apply(state, &path)?;
    Ok(path)
}

/// Returns a cached thumbnail for a wallpaper, generating it on first use.
///
/// The path is hashed rather than reused so that two files with the same name in
/// different folders do not collide in the cache.
pub fn thumbnail(path: &str, size: u32) -> Result<PathBuf, String> {
    let cache = bw_core::paths::thumbnail_dir();
    std::fs::create_dir_all(&cache)
        .map_err(|error| format!("could not create the thumbnail cache: {error}"))?;

    let target = cache.join(format!("{}-{size}.jpg", hash_path(path)));
    if target.exists() {
        return Ok(target);
    }

    let image = image::open(path).map_err(|error| format!("could not read {path}: {error}"))?;
    // Thumbnails are for a grid, so filling the tile matters more than showing
    // the whole image.
    image
        .resize_to_fill(size, size * 10 / 16, FilterType::Triangle)
        .into_rgb8()
        .save(&target)
        .map_err(|error| format!("could not write the thumbnail: {error}"))?;

    Ok(target)
}

fn hash_path(path: &str) -> String {
    // FNV-1a: not cryptographic, but stable across runs, which is all a cache
    // filename needs.
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in path.to_lowercase().bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

/// Searches one of the online providers.
pub async fn search_online(
    state: &AppState,
    provider: &str,
    search: &str,
    page: u32,
) -> Result<WallpaperPage, String> {
    let config = state.config();
    let provider = Provider::parse(provider).map_err(|error| error.to_string())?;

    let query = Query {
        provider,
        search: search.to_owned(),
        page: page.max(1),
        resolution: config.wallpaper_selector.online.resolution.clone(),
        purity: config.wallpaper_selector.online.purity.clone(),
        category: config.wallpaper_selector.online.category.clone(),
        // A seed stable for the session keeps "random" ordering consistent while
        // the user pages through it.
        seed: session_seed(),
    };

    let url = online::request_url(&query);
    let client = reqwest::Client::builder()
        .user_agent(concat!("beautiful-wallpaper/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| error.to_string())?;

    let mut request = client.get(&url);
    if provider.requires_key() {
        let key = api_key(provider).ok_or_else(|| {
            format!(
                "{} needs an API key; add one in Settings",
                provider.as_str()
            )
        })?;
        request = match provider {
            Provider::Unsplash => request.header("Authorization", format!("Client-ID {key}")),
            Provider::Pexels => request.header("Authorization", key),
            Provider::Wallhaven => request,
        };
    }

    let response = request
        .send()
        .await
        .map_err(|error| format!("could not reach {}: {error}", provider.as_str()))?;

    if !response.status().is_success() {
        return Err(format!(
            "{} answered {}",
            provider.as_str(),
            response.status()
        ));
    }

    let body = response
        .text()
        .await
        .map_err(|error| format!("could not read the response: {error}"))?;

    online::parse_page(provider, &body, query.page).map_err(|error| error.to_string())
}

/// Downloads a wallpaper into the configured folder and returns its path.
pub async fn download(
    state: &AppState,
    url: &str,
    provider: &str,
    download_location: &str,
) -> Result<String, String> {
    let config = state.config();
    let folder = if config.wallpaper_selector.online.download_path.is_empty() {
        bw_core::paths::default_download_dir()
    } else {
        PathBuf::from(&config.wallpaper_selector.online.download_path)
    };
    std::fs::create_dir_all(&folder)
        .map_err(|error| format!("could not create {}: {error}", folder.display()))?;

    let name = url
        .rsplit('/')
        .next()
        .filter(|segment| !segment.is_empty())
        .unwrap_or("wallpaper.jpg");
    let target = folder.join(sanitise(name));

    let client = reqwest::Client::builder()
        .user_agent(concat!("beautiful-wallpaper/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| error.to_string())?;

    let bytes = client
        .get(url)
        .send()
        .await
        .map_err(|error| format!("could not download: {error}"))?
        .bytes()
        .await
        .map_err(|error| format!("could not read the download: {error}"))?;

    let mut file = std::fs::File::create(&target)
        .map_err(|error| format!("could not create {}: {error}", target.display()))?;
    file.write_all(&bytes)
        .map_err(|error| format!("could not write {}: {error}", target.display()))?;

    // Unsplash's API terms require a download to be reported; it is a fire and
    // forget ping, so a failure must not fail the download itself.
    if provider == "unsplash" && !download_location.is_empty() {
        if let Some(key) = api_key(Provider::Unsplash) {
            let _ = client
                .get(download_location)
                .header("Authorization", format!("Client-ID {key}"))
                .send()
                .await;
        }
    }

    Ok(target.to_string_lossy().into_owned())
}

fn sanitise(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            other => other,
        })
        .collect()
}

/// Reads a provider's API key from the Windows Credential Manager.
///
/// The counterpart of the original's `secret-tool` lookups. A missing key is not
/// an error here — the caller turns it into a message the UI can show.
fn api_key(provider: Provider) -> Option<String> {
    let entry = keyring::Entry::new("beautiful-wallpaper", provider.as_str()).ok()?;
    entry.get_password().ok()
}

/// Stores a provider's API key.
pub fn set_api_key(provider: &str, key: &str) -> Result<(), String> {
    let provider = Provider::parse(provider).map_err(|error| error.to_string())?;
    let entry = keyring::Entry::new("beautiful-wallpaper", provider.as_str())
        .map_err(|error| format!("could not open the credential store: {error}"))?;

    if key.is_empty() {
        // An empty key means "forget it"; deleting a key that was never stored
        // is not a failure.
        let _ = entry.delete_credential();
        return Ok(());
    }
    entry
        .set_password(key)
        .map_err(|error| format!("could not store the key: {error}"))
}

fn session_seed() -> String {
    use std::sync::OnceLock;
    static SEED: OnceLock<String> = OnceLock::new();
    SEED.get_or_init(|| {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.subsec_nanos())
            .unwrap_or_default();
        format!("bw{nanos:08x}")
    })
    .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thumbnail_names_are_stable_and_case_insensitive() {
        assert_eq!(hash_path(r"C:\Pics\a.png"), hash_path(r"c:\pics\A.PNG"));
        assert_ne!(hash_path(r"C:\Pics\a.png"), hash_path(r"C:\Other\a.png"));
        assert_eq!(hash_path("x").len(), 16);
    }

    #[test]
    fn download_names_drop_characters_windows_rejects() {
        assert_eq!(sanitise("a<b>c:d.jpg"), "a_b_c_d.jpg");
        assert_eq!(sanitise("plain.jpg"), "plain.jpg");
    }

    #[test]
    fn the_session_seed_does_not_change_between_calls() {
        assert_eq!(session_seed(), session_seed());
    }
}
