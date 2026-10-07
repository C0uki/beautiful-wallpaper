//! The weather for the bar and the desktop widget, from wttr.in.
//!
//! Both listened for it from the start, and nothing ever sent it: they said the
//! weather was unavailable forever.

use std::time::{Duration, Instant};

use bw_core::weather::WeatherState;
use tauri::{AppHandle, Emitter, Manager};

use crate::commands::event;
use crate::state::AppState;

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
                tried = asking;
                match fetch(&client, &weather.city, weather.use_usc_units, &lang).await {
                    Ok(fetched) => {
                        report = Some(fetched);
                        let every = weather.refresh_interval.max(60);
                        next = Instant::now() + Duration::from_secs(every.into());
                    }
                    Err(error) => {
                        // Offline, or wttr.in having a bad minute: both
                        // ordinary, and both answered by trying again soon.
                        tracing::warn!(%error, "could not fetch the weather");
                        next = Instant::now() + Duration::from_secs(60);
                    }
                }
            }
            if let (true, Some(report)) = (weather.enable, &report) {
                let _ = app.emit(event::WEATHER, report);
            }
            tokio::time::sleep(Duration::from_secs(10)).await;
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
) -> Result<WeatherState, String> {
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
        .ok_or_else(|| format!("wttr.in did not send a weather report for {city:?}"))
}
