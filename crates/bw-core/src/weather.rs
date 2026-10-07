//! The weather, as wttr.in reports it.
//!
//! wttr.in is what end4-pC's own weather service reads: no key, a city name or
//! none at all (it then places the caller by IP), and one JSON document
//! (`?format=j1`) holding everything the bar and the desktop widget show.

use serde::Serialize;
use serde_json::Value;

/// What the bar and the desktop widget show: `WeatherState` in
/// `packages/core/src/ipc.ts`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeatherState {
    pub city: String,
    pub description: String,
    /// Celsius, or Fahrenheit with `weather.useUscUnits`.
    pub temperature: f64,
    pub humidity: f64,
    /// Metres per second, which is what the widgets label it.
    pub wind_speed: f64,
    /// A Material Symbols name the bundled font has.
    pub icon: String,
    pub sunrise: String,
    pub sunset: String,
}

/// Reads a `?format=j1` document. `lang` is the language asked for with
/// `&lang=`, whose description wttr.in adds as `lang_<lang>`; without one the
/// English description is used.
pub fn parse(body: &str, usc: bool, lang: &str) -> Option<WeatherState> {
    let root: Value = serde_json::from_str(body).ok()?;
    let now = root.get("current_condition")?.get(0)?;
    let text = |node: &Value, key: &str| Some(node.get(key)?.as_str()?.trim().to_owned());
    let first = |node: &Value, key: &str| text(node.get(key)?.get(0)?, "value");
    let number = |key: &str| text(now, key)?.parse::<f64>().ok();
    let astronomy = root
        .get("weather")
        .and_then(|days| days.get(0)?.get("astronomy")?.get(0));
    let sun = |key: &str| astronomy.and_then(|day| text(day, key)).unwrap_or_default();
    // The report says itself whether it is night where it was taken: its
    // icon is the night one. The times next to it cannot say as much — the
    // observation is in UTC, and sunrise and sunset are local.
    let night = first(now, "weatherIconUrl").is_some_and(|url| url.contains("night"));

    Some(WeatherState {
        city: root
            .get("nearest_area")
            .and_then(|areas| first(areas.get(0)?, "areaName"))
            .unwrap_or_default(),
        description: first(now, &format!("lang_{lang}"))
            .or_else(|| first(now, "weatherDesc"))
            .unwrap_or_default(),
        temperature: number(if usc { "temp_F" } else { "temp_C" })?,
        humidity: number("humidity").unwrap_or_default(),
        wind_speed: number("windspeedKmph").unwrap_or_default() / 3.6,
        icon: icon(number("weatherCode").unwrap_or_default() as u32, night).to_owned(),
        sunrise: sun("sunrise"),
        sunset: sun("sunset"),
    })
}

/// The icon for one of the World Weather Online condition codes wttr.in uses.
fn icon(code: u32, night: bool) -> &'static str {
    match code {
        113 if night => "clear_night",
        113 => "sunny",
        116 => "cloud",
        119 | 122 => "cloudy",
        143 | 248 | 260 => "foggy",
        200 | 386..=395 => "thunderstorm",
        176 | 263 | 266 | 293..=308 | 353..=359 => "rainy",
        179..=185 | 227 | 230 | 281 | 284 | 311..=338 | 350 | 362..=377 => "weather_snowy",
        _ => "cloud",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Trimmed from a real `wttr.in/komaki?format=j1&lang=ja` response.
    const KOMAKI: &str = r#"{"current_condition": [{"temp_C": "20", "temp_F": "68", "humidity": "74", "windspeedKmph": "6", "weatherCode": "122", "lang_ja": [{"value": "曇り"}], "weatherDesc": [{"value": "Overcast "}]}], "nearest_area": [{"areaName": [{"value": "Komaki"}]}], "weather": [{"astronomy": [{"sunrise": "05:49 AM", "sunset": "05:33 PM"}]}]}"#;

    #[test]
    fn reads_a_real_response() {
        let weather = parse(KOMAKI, false, "ja").expect("parses");
        assert_eq!(weather.city, "Komaki");
        assert_eq!(weather.description, "曇り");
        assert_eq!(weather.temperature, 20.0);
        assert_eq!(weather.humidity, 74.0);
        assert!((weather.wind_speed - 6.0 / 3.6).abs() < 1e-9);
        assert_eq!(weather.icon, "cloudy");
        assert_eq!(weather.sunset, "05:33 PM");

        let english = parse(KOMAKI, true, "").expect("parses");
        assert_eq!(english.description, "Overcast");
        assert_eq!(english.temperature, 68.0);
    }

    /// Trimmed from a real `wttr.in/New+York?format=j1`, at night there.
    const NEW_YORK_NIGHT: &str = r#"{"current_condition": [{"temp_C": "17", "temp_F": "63", "humidity": "70", "windspeedKmph": "9", "weatherCode": "113", "weatherIconUrl": [{"value": "https://cdn.worldweatheronline.com/images/wsymbols01_png_64/wsymbol_0008_clear_sky_night.png"}], "weatherDesc": [{"value": "Clear "}]}]}"#;

    #[test]
    fn a_clear_night_is_not_sunny() {
        let weather = parse(NEW_YORK_NIGHT, false, "").expect("parses");
        assert_eq!(weather.icon, "clear_night");
        // Without the night icon, as in the trimmed Komaki report: day.
        assert_eq!(parse(KOMAKI, false, "").expect("parses").icon, "cloudy");
        assert_eq!(icon(113, false), "sunny");
    }

    #[test]
    fn refuses_what_is_not_a_report() {
        assert_eq!(parse("Unknown location", false, ""), None);
        assert_eq!(parse("{}", false, ""), None);
    }
}
