//! Lyrics for the playing track, from lrclib.net.
//!
//! end4-pC fetched them with an external script. lrclib.net is a free lyrics
//! database with a JSON API and no key, and most of its entries carry timed
//! lines (LRC) besides the plain text, which is what lets the media tab follow
//! the song.

use serde::Serialize;
use serde_json::Value;

/// One line, and when in the track it is sung.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LyricLine {
    /// Seconds from the start of the track.
    pub time: f64,
    /// Empty for a gap between verses.
    pub text: String,
}

/// What the media tab shows: `Lyrics` in `packages/core/src/ipc.ts`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Lyrics {
    /// Timed lines in order; empty when only the plain text is known.
    pub lines: Vec<LyricLine>,
    pub plain: String,
}

/// Reads one lrclib.net record — an `/api/get` answer, or an element of an
/// `/api/search` one. `None` for an instrumental, or a record with no text.
pub fn parse(record: &Value) -> Option<Lyrics> {
    let text = |key: &str| record.get(key).and_then(Value::as_str).unwrap_or_default();
    let lyrics = Lyrics {
        lines: timed(text("syncedLyrics")),
        plain: text("plainLyrics").trim().to_owned(),
    };
    (!lyrics.lines.is_empty() || !lyrics.plain.is_empty()).then_some(lyrics)
}

/// The lines of an LRC document, in time order.
///
/// Each line is `[mm:ss.xx]text`, and may carry several stamps when it is
/// sung more than once. Tags that are not times — `[ar:…]`, `[offset:…]` —
/// are skipped along with their line.
pub fn timed(lrc: &str) -> Vec<LyricLine> {
    let mut lines = Vec::new();
    for line in lrc.lines() {
        let mut rest = line.trim();
        let mut stamps = Vec::new();
        while let Some(after) = rest.strip_prefix('[') {
            let Some(end) = after.find(']') else {
                break;
            };
            let Some(time) = stamp(&after[..end]) else {
                break;
            };
            stamps.push(time);
            rest = &after[end + 1..];
        }
        for time in stamps {
            lines.push(LyricLine {
                time,
                text: rest.trim().to_owned(),
            });
        }
    }
    lines.sort_by(|a, b| a.time.total_cmp(&b.time));
    lines
}

fn stamp(tag: &str) -> Option<f64> {
    let (minutes, seconds) = tag.split_once(':')?;
    let minutes = minutes.trim().parse::<f64>().ok()?;
    let seconds = seconds.trim().parse::<f64>().ok()?;
    Some(minutes * 60.0 + seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_timed_lines_in_order() {
        let lines =
            timed("[ar:Someone]\n[00:12.50]first\n[01:02.00][00:20.00]twice\n[00:30.00]\nno stamp");
        let read: Vec<_> = lines
            .iter()
            .map(|line| (line.time, line.text.as_str()))
            .collect();
        assert_eq!(
            read,
            [
                (12.5, "first"),
                (20.0, "twice"),
                (30.0, ""),
                (62.0, "twice")
            ]
        );
    }

    #[test]
    fn reads_a_record_and_refuses_an_empty_one() {
        let record = serde_json::json!({
            "syncedLyrics": "[00:01.00]one\n[00:02.00]two",
            "plainLyrics": "one\ntwo\n",
        });
        let lyrics = parse(&record).expect("has lyrics");
        assert_eq!(lyrics.lines.len(), 2);
        assert_eq!(lyrics.plain, "one\ntwo");

        let plain_only = serde_json::json!({ "syncedLyrics": null, "plainLyrics": "words" });
        assert!(parse(&plain_only).expect("has words").lines.is_empty());

        let instrumental =
            serde_json::json!({ "instrumental": true, "syncedLyrics": null, "plainLyrics": null });
        assert_eq!(parse(&instrumental), None);
    }
}
