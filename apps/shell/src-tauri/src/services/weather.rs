//! The weather for the bar and the desktop widget, from wttr.in.
//!
//! Both listened for it from the start, and nothing ever sent it: they said the
//! weather was unavailable forever.

use std::time::{Duration, Instant};

use bw_core::weather::WeatherState;
use tauri::{AppHandle, Emitter, Manager};

use crate::commands::event;
use crate::state::AppState;

/// Where the last report wttr.in sent is kept between runs: what was asked on
/// the first line, the document itself after it.
fn cache_path() -> std::path::PathBuf {
    bw_core::paths::cache_dir().join("weather.txt")
}

/// The report kept from the last run, if it answers the same question.
///
/// Startup is when wttr.in is likeliest to fail — the shell starts with the
/// session, often before the network is up — and without this the widget and
/// the bar said the weather was unavailable until the first fetch got
/// through. An hour-old report is a better thing to show than none.
fn cached(asking: &str, usc: bool, lang: &str) -> Option<WeatherState> {
    let kept = std::fs::read_to_string(cache_path()).ok()?;
    let (asked, body) = kept.split_once('\n')?;
    if asked != asking {
        return None;
    }
    bw_core::weather::parse(body, usc, lang)
}

/// How long to wait after the `failures`-th failure in a row: soon at first,
/// since one failure is usually a moment's trouble, then up to a minute.
fn retry_after(failures: u32) -> Duration {
    Duration::from_secs((10u64 << failures.saturating_sub(1).min(3)).min(60))
}

/// Fetches every `weather.refreshInterval` seconds, and at once when the city,
/// the units or the language change. The last report is sent again every few
/// seconds besides: a surface that has just loaded has to be told what is
/// already known, rather than wait out the interval saying it knows nothing.
pub fn start(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();
        let mut report: Option<WeatherState> = None;
        let mut tried = String::new();
        let mut next = Instant::now();
        let mut failures = 0;
        loop {
            let config = app.state::<AppState>().config();
            let weather = &config.weather;
            // `ja_JP` asks for Japanese, and `auto` for the language the
            // surfaces resolve it to: the system's.
            let ui = match config.language.ui.as_str() {
                "auto" => system_language(),
                chosen => chosen.to_owned(),
            };
            let lang = ui.split(['_', '-']).next().unwrap_or_default().to_owned();
            let asking = format!("{}|{}|{lang}", weather.city, weather.use_usc_units);

            if weather.enable && (Instant::now() >= next || asking != tried) {
                if asking != tried {
                    // Something to show while the first fetch is under way —
                    // sent now, not after it: wttr.in can take its full
                    // timeout to answer, and the bar stayed empty that long.
                    report = cached(&asking, weather.use_usc_units, &lang);
                    if let Some(report) = &report {
                        crate::commands::emit_reading(&app, event::WEATHER, report);
                    }
                }
                tried = asking.clone();
                // A page that loads while this is under way asks for the kept
                // report itself, through `latest_readings`.
                let fetched = fetch(&client, &weather.city, weather.use_usc_units, &lang).await;
                match fetched {
                    Ok((fetched, body)) => {
                        report = Some(fetched);
                        failures = 0;
                        let every = weather.refresh_interval.max(60);
                        next = Instant::now() + Duration::from_secs(every.into());
                        let _ = std::fs::create_dir_all(bw_core::paths::cache_dir());
                        let _ = std::fs::write(cache_path(), format!("{asking}\n{body}"));
                    }
                    Err(error) => {
                        // Offline, or wttr.in having a bad minute: both
                        // ordinary, and both answered by trying again soon.
                        tracing::warn!(%error, "could not fetch the weather");
                        failures += 1;
                        next = Instant::now() + retry_after(failures);
                    }
                }
            }
            if let (true, Some(report)) = (weather.enable, &report) {
                crate::commands::emit_reading(&app, event::WEATHER, report);
            }
            // Only sent when it is new, so this is just how soon a changed
            // city is noticed.
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });
}

/// The first of the person's preferred languages, as a BCP-47 tag such as
/// `ja-JP`: what WebView2 hands the surfaces as `navigator.languages`. Empty
/// when Windows will not say, which asks wttr.in for English.
#[cfg(windows)]
fn system_language() -> String {
    windows::Globalization::ApplicationLanguages::Languages()
        .and_then(|languages| languages.GetAt(0))
        .map(|tag| tag.to_string())
        .unwrap_or_default()
}

#[cfg(not(windows))]
fn system_language() -> String {
    String::new()
}

async fn fetch(
    client: &reqwest::Client,
    city: &str,
    usc: bool,
    lang: &str,
) -> Result<(WeatherState, String), String> {
    // An empty city leaves the path empty, and wttr.in places the request by
    // its address instead.
    let mut url = reqwest::Url::parse("https://wttr.in/").expect("a valid address");
    url.path_segments_mut()
        .map_err(|()| "wttr.in's address takes no path".to_owned())?
        .pop_if_empty()
        .push(city);
    url.query_pairs_mut().append_pair("format", "j1");
    if !lang.is_empty() {
        url.query_pairs_mut().append_pair("lang", lang);
    }

    let body = client
        .get(url)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|error| error.to_string())?
        .text()
        .await
        .map_err(|error| error.to_string())?;
    bw_core::weather::parse(&body, usc, lang)
        .map(|report| (report, body))
        .ok_or_else(|| format!("wttr.in did not send a weather report for {city:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retries_come_soon_and_then_settle_at_a_minute() {
        let waits: Vec<u64> = (1..=6).map(|n| retry_after(n).as_secs()).collect();
        assert_eq!(waits, [10, 20, 40, 60, 60, 60]);
    }
}
