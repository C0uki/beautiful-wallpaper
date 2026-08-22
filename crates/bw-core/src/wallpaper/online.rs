//! Wallhaven, Unsplash and Pexels, normalised to one shape.
//!
//! A port of end4-pC's `services/OnlineWallpapers.qml`. Only the pure half lives
//! here — building request URLs and parsing responses — so it is testable without
//! a network. The shell crate performs the actual requests.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use ts_rs::TS;

#[derive(Debug, thiserror::Error)]
pub enum OnlineError {
    #[error("`{0}` is not a known wallpaper provider")]
    UnknownProvider(String),
    #[error("{provider} returned something unexpected: {detail}")]
    BadResponse {
        provider: &'static str,
        detail: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export)]
pub enum Provider {
    Wallhaven,
    Unsplash,
    Pexels,
}

impl Provider {
    pub fn parse(name: &str) -> Result<Self, OnlineError> {
        match name.to_ascii_lowercase().as_str() {
            "wallhaven" => Ok(Provider::Wallhaven),
            "unsplash" => Ok(Provider::Unsplash),
            "pexels" => Ok(Provider::Pexels),
            _ => Err(OnlineError::UnknownProvider(name.to_owned())),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Provider::Wallhaven => "wallhaven",
            Provider::Unsplash => "unsplash",
            Provider::Pexels => "pexels",
        }
    }

    /// Whether this provider refuses to answer without an API key.
    pub fn requires_key(self) -> bool {
        !matches!(self, Provider::Wallhaven)
    }
}

/// One result, in the shape every provider is flattened into.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WallpaperItem {
    pub id: String,
    /// Grid thumbnail.
    pub thumb: String,
    /// Full-resolution download.
    pub full: String,
    pub provider: Provider,
    pub title: String,
    pub author: String,
    pub author_url: String,
    pub likes: u32,
    pub width: u32,
    pub height: u32,
    /// Unsplash requires a download to be reported back to this endpoint; empty
    /// for providers that have no such requirement.
    pub download_location: String,
}

/// A page of results.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct WallpaperPage {
    pub items: Vec<WallpaperItem>,
    pub page: u32,
    pub total_pages: u32,
}

/// What to ask a provider for.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct Query {
    pub provider: Provider,
    pub search: String,
    pub page: u32,
    /// `"1080p"` | `"2k"` | `"4k"`
    pub resolution: String,
    /// Wallhaven only: `"sfw"` | `"sketchy"` | `"nsfw"`.
    pub purity: String,
    pub category: String,
    /// Keeps "random" ordering stable while paging through it.
    pub seed: String,
}

impl Default for Query {
    fn default() -> Self {
        Self {
            provider: Provider::Wallhaven,
            search: String::new(),
            page: 1,
            resolution: "1080p".into(),
            purity: "sfw".into(),
            category: "general".into(),
            seed: String::new(),
        }
    }
}

/// Minimum resolution, as `WIDTHxHEIGHT`.
pub fn resolution_dimensions(name: &str) -> (u32, u32) {
    match name {
        "4k" => (3840, 2160),
        "2k" => (2560, 1440),
        _ => (1920, 1080),
    }
}

/// Wallhaven's three-digit purity mask: sfw / sketchy / nsfw.
fn wallhaven_purity(name: &str) -> &'static str {
    match name {
        "nsfw" => "001",
        "sketchy" => "010",
        _ => "100",
    }
}

/// Wallhaven's three-digit category mask: general / anime / people.
fn wallhaven_categories(name: &str) -> &'static str {
    match name {
        "anime" => "010",
        "people" => "001",
        "all" => "111",
        _ => "100",
    }
}

fn encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            b' ' => out.push_str("%20"),
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

/// Builds the request URL for a query. The API key, where one is needed, is sent
/// as a header by the caller rather than baked into the URL.
pub fn request_url(query: &Query) -> String {
    let (width, height) = resolution_dimensions(&query.resolution);
    match query.provider {
        Provider::Wallhaven => {
            let mut url = format!(
                "https://wallhaven.cc/api/v1/search?atleast={width}x{height}&purity={}&categories={}&page={}",
                wallhaven_purity(&query.purity),
                wallhaven_categories(&query.category),
                query.page,
            );
            if query.search.is_empty() {
                // Without a search term, ask for a stable random ordering so
                // paging does not return the same wallpapers twice.
                url.push_str("&sorting=random");
                if !query.seed.is_empty() {
                    url.push_str(&format!("&seed={}", encode(&query.seed)));
                }
            } else {
                url.push_str(&format!("&q={}", encode(&query.search)));
            }
            url
        }
        Provider::Unsplash => {
            if query.search.is_empty() {
                format!(
                    "https://api.unsplash.com/photos?per_page=30&order_by=popular&page={}",
                    query.page
                )
            } else {
                format!(
                    "https://api.unsplash.com/search/photos?per_page=30&query={}&page={}",
                    encode(&query.search),
                    query.page
                )
            }
        }
        Provider::Pexels => {
            if query.search.is_empty() {
                format!(
                    "https://api.pexels.com/v1/curated?per_page=30&page={}",
                    query.page
                )
            } else {
                format!(
                    "https://api.pexels.com/v1/search?per_page=30&query={}&page={}",
                    encode(&query.search),
                    query.page
                )
            }
        }
    }
}

/// Parses a provider's response into the common shape.
pub fn parse_page(provider: Provider, body: &str, page: u32) -> Result<WallpaperPage, OnlineError> {
    let root: Value = serde_json::from_str(body).map_err(|source| OnlineError::BadResponse {
        provider: provider.as_str(),
        detail: source.to_string(),
    })?;

    match provider {
        Provider::Wallhaven => parse_wallhaven(&root, page),
        Provider::Unsplash => parse_unsplash(&root, page),
        Provider::Pexels => parse_pexels(&root, page),
    }
}

fn bad(provider: &'static str, detail: impl Into<String>) -> OnlineError {
    OnlineError::BadResponse {
        provider,
        detail: detail.into(),
    }
}

fn text(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn number(value: &Value, key: &str) -> u64 {
    value.get(key).and_then(Value::as_u64).unwrap_or_default()
}

fn parse_wallhaven(root: &Value, page: u32) -> Result<WallpaperPage, OnlineError> {
    let data = root
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| bad("wallhaven", "no `data` array"))?;

    let items = data
        .iter()
        .map(|item| {
            let dimensions = |key: &str| number(item, key) as u32;
            WallpaperItem {
                id: text(item, "id"),
                thumb: item
                    .get("thumbs")
                    .map(|t| text(t, "large"))
                    .unwrap_or_default(),
                full: text(item, "path"),
                provider: Provider::Wallhaven,
                // Wallhaven has no titles; its uploader field is often absent
                // too, so fall back to the id rather than showing nothing.
                title: text(item, "id"),
                author: String::new(),
                author_url: text(item, "url"),
                likes: number(item, "favorites") as u32,
                width: dimensions("dimension_x"),
                height: dimensions("dimension_y"),
                download_location: String::new(),
            }
        })
        .collect();

    let total_pages = root
        .get("meta")
        .map(|meta| number(meta, "last_page") as u32)
        .unwrap_or(page);

    Ok(WallpaperPage {
        items,
        page,
        total_pages: total_pages.max(page),
    })
}

fn parse_unsplash(root: &Value, page: u32) -> Result<WallpaperPage, OnlineError> {
    // `/photos` returns a bare array, `/search/photos` wraps it in `results`.
    let (data, total_pages) = match root {
        Value::Array(items) => (items.clone(), page),
        Value::Object(_) => {
            let results = root
                .get("results")
                .and_then(Value::as_array)
                .ok_or_else(|| bad("unsplash", "no `results` array"))?;
            (results.clone(), number(root, "total_pages") as u32)
        }
        _ => return Err(bad("unsplash", "expected an object or an array")),
    };

    let items = data
        .iter()
        .map(|item| {
            let urls = item.get("urls");
            let user = item.get("user");
            WallpaperItem {
                id: text(item, "id"),
                thumb: urls.map(|u| text(u, "small")).unwrap_or_default(),
                full: urls.map(|u| text(u, "full")).unwrap_or_default(),
                provider: Provider::Unsplash,
                title: {
                    // Unsplash leaves `description` null more often than not.
                    let description = text(item, "description");
                    if description.is_empty() {
                        text(item, "alt_description")
                    } else {
                        description
                    }
                },
                author: user.map(|u| text(u, "name")).unwrap_or_default(),
                author_url: user
                    .and_then(|u| u.get("links"))
                    .map(|l| text(l, "html"))
                    .unwrap_or_default(),
                likes: number(item, "likes") as u32,
                width: number(item, "width") as u32,
                height: number(item, "height") as u32,
                download_location: item
                    .get("links")
                    .map(|l| text(l, "download_location"))
                    .unwrap_or_default(),
            }
        })
        .collect();

    Ok(WallpaperPage {
        items,
        page,
        total_pages: total_pages.max(page),
    })
}

fn parse_pexels(root: &Value, page: u32) -> Result<WallpaperPage, OnlineError> {
    let data = root
        .get("photos")
        .and_then(Value::as_array)
        .ok_or_else(|| bad("pexels", "no `photos` array"))?;

    let items = data
        .iter()
        .map(|item| {
            let src = item.get("src");
            WallpaperItem {
                id: number(item, "id").to_string(),
                thumb: src.map(|s| text(s, "medium")).unwrap_or_default(),
                full: src.map(|s| text(s, "original")).unwrap_or_default(),
                provider: Provider::Pexels,
                title: text(item, "alt"),
                author: text(item, "photographer"),
                author_url: text(item, "photographer_url"),
                likes: 0,
                width: number(item, "width") as u32,
                height: number(item, "height") as u32,
                download_location: String::new(),
            }
        })
        .collect();

    let per_page = number(root, "per_page").max(1);
    let total_pages = number(root, "total_results").div_ceil(per_page) as u32;

    Ok(WallpaperPage {
        items,
        page,
        total_pages: total_pages.max(page),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_names_round_trip() {
        for name in ["wallhaven", "unsplash", "pexels"] {
            assert_eq!(Provider::parse(name).unwrap().as_str(), name);
        }
        assert_eq!(Provider::parse("WALLHAVEN").unwrap(), Provider::Wallhaven);
        assert!(Provider::parse("bing").is_err());
    }

    #[test]
    fn only_wallhaven_works_without_a_key() {
        assert!(!Provider::Wallhaven.requires_key());
        assert!(Provider::Unsplash.requires_key());
        assert!(Provider::Pexels.requires_key());
    }

    #[test]
    fn wallhaven_url_encodes_filters_and_search() {
        let query = Query {
            search: "night sky".into(),
            resolution: "4k".into(),
            purity: "sfw".into(),
            category: "anime".into(),
            page: 3,
            ..Query::default()
        };
        let url = request_url(&query);
        assert!(url.contains("atleast=3840x2160"), "{url}");
        assert!(url.contains("purity=100"), "{url}");
        assert!(url.contains("categories=010"), "{url}");
        assert!(url.contains("q=night%20sky"), "{url}");
        assert!(url.contains("page=3"), "{url}");
        assert!(!url.contains("sorting=random"), "{url}");
    }

    #[test]
    fn an_empty_search_asks_wallhaven_for_a_seeded_random_order() {
        let query = Query {
            seed: "abc123".into(),
            ..Query::default()
        };
        let url = request_url(&query);
        assert!(url.contains("sorting=random"), "{url}");
        assert!(url.contains("seed=abc123"), "{url}");
    }

    #[test]
    fn unsplash_and_pexels_switch_endpoint_on_search() {
        let browse = Query {
            provider: Provider::Unsplash,
            ..Query::default()
        };
        assert!(request_url(&browse).contains("/photos?"));
        let search = Query {
            search: "forest".into(),
            ..browse.clone()
        };
        assert!(request_url(&search).contains("/search/photos?"));

        let curated = Query {
            provider: Provider::Pexels,
            ..Query::default()
        };
        assert!(request_url(&curated).contains("/curated?"));
    }

    #[test]
    fn wallhaven_responses_normalise() {
        let body = r#"{
          "data": [{
            "id": "abc123",
            "url": "https://wallhaven.cc/w/abc123",
            "path": "https://w.wallhaven.cc/full/ab/abc123.jpg",
            "favorites": 42,
            "dimension_x": 3840,
            "dimension_y": 2160,
            "thumbs": { "large": "https://th.wallhaven.cc/lg/ab/abc123.jpg" }
          }],
          "meta": { "last_page": 12 }
        }"#;
        let page = parse_page(Provider::Wallhaven, body, 1).unwrap();
        let item = &page.items[0];
        assert_eq!(item.id, "abc123");
        assert_eq!(item.provider, Provider::Wallhaven);
        assert_eq!(item.full, "https://w.wallhaven.cc/full/ab/abc123.jpg");
        assert_eq!(item.thumb, "https://th.wallhaven.cc/lg/ab/abc123.jpg");
        assert_eq!((item.width, item.height), (3840, 2160));
        assert_eq!(item.likes, 42);
        assert_eq!(page.total_pages, 12);
    }

    #[test]
    fn unsplash_search_and_browse_shapes_both_parse() {
        let search = r#"{
          "total_pages": 5,
          "results": [{
            "id": "xyz",
            "description": "A lake",
            "likes": 7,
            "width": 6000, "height": 4000,
            "urls": { "small": "s.jpg", "full": "f.jpg" },
            "links": { "download_location": "https://api.unsplash.com/photos/xyz/download" },
            "user": { "name": "Ada", "links": { "html": "https://unsplash.com/@ada" } }
          }]
        }"#;
        let page = parse_page(Provider::Unsplash, search, 2).unwrap();
        let item = &page.items[0];
        assert_eq!(item.title, "A lake");
        assert_eq!(item.author, "Ada");
        assert_eq!(item.author_url, "https://unsplash.com/@ada");
        assert!(item.download_location.ends_with("/download"));
        assert_eq!(page.total_pages, 5);

        let browse = r#"[{"id":"q","urls":{"small":"s","full":"f"},"width":1,"height":2}]"#;
        let page = parse_page(Provider::Unsplash, browse, 1).unwrap();
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].id, "q");
    }

    #[test]
    fn unsplash_falls_back_to_alt_description_for_a_title() {
        let body =
            r#"[{"id":"q","description":null,"alt_description":"a misty pine forest","urls":{}}]"#;
        let page = parse_page(Provider::Unsplash, body, 1).unwrap();
        assert_eq!(page.items[0].title, "a misty pine forest");
    }

    #[test]
    fn pexels_responses_normalise_and_compute_page_count() {
        let body = r#"{
          "page": 1, "per_page": 30, "total_results": 61,
          "photos": [{
            "id": 99, "alt": "Dunes", "photographer": "Bo",
            "photographer_url": "https://pexels.com/@bo",
            "width": 5000, "height": 3000,
            "src": { "medium": "m.jpg", "original": "o.jpg" }
          }]
        }"#;
        let page = parse_page(Provider::Pexels, body, 1).unwrap();
        assert_eq!(page.items[0].id, "99");
        assert_eq!(page.items[0].author, "Bo");
        assert_eq!(page.items[0].full, "o.jpg");
        // 61 results at 30 per page is 3 pages, not 2.
        assert_eq!(page.total_pages, 3);
    }

    #[test]
    fn malformed_responses_report_the_provider() {
        let error = parse_page(Provider::Pexels, r#"{"nope":1}"#, 1).unwrap_err();
        assert!(error.to_string().contains("pexels"), "{error}");
        let error = parse_page(Provider::Wallhaven, "not json", 1).unwrap_err();
        assert!(error.to_string().contains("wallhaven"), "{error}");
    }

    #[test]
    fn missing_fields_degrade_to_empty_rather_than_panicking() {
        let page = parse_page(Provider::Wallhaven, r#"{"data":[{}]}"#, 1).unwrap();
        let item = &page.items[0];
        assert_eq!(item.id, "");
        assert_eq!(item.likes, 0);
        assert_eq!(page.total_pages, 1);
    }
}
