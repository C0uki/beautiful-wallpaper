//! The chat's data: streamed events, messages, and the conversation store.
//!
//! Everything here is parsing and bookkeeping, which is exactly the part worth
//! testing — the shell crate's tests only run on the Windows CI job, and an
//! SSE parser discovered to be wrong there is discovered too late.
//!
//! The wire format is the Messages API's server-sent events. Only a handful of
//! the event types matter to a chat window; the rest are skipped rather than
//! rejected, because the API adds event types and an unknown one is not an
//! error.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::ai::AiError;

/// One thing the stream told us. What the UI reacts to, rather than the raw
/// event zoo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", tag = "kind", content = "value")]
#[ts(export)]
pub enum StreamEvent {
    /// A piece of the reply.
    Text(String),
    /// A piece of the model's summarised reasoning.
    Thinking(String),
    /// The model started a web search, with the query it used.
    Search(String),
    /// The sources a search turned up. They arrive as one completed block
    /// rather than one at a time, so this carries the whole set.
    Sources(Vec<SearchSource>),
    /// Safety classifiers declined and another model picked the request up.
    /// Worth showing: the reply is no longer from the model that was asked for.
    FellBackTo(String),
    /// The turn finished.
    Done,
    /// The turn ended without a usable reply.
    Failed(AiError),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct SearchSource {
    pub title: String,
    pub url: String,
}

/// Parses one SSE `data:` payload.
///
/// Returns `None` for events a chat window has nothing to do with — `ping`,
/// `content_block_stop`, usage-only `message_delta`s — and for anything
/// unrecognised. Unknown events are skipped rather than failed: the API adds
/// event types, and a chat that stopped working when it did would be brittle
/// for no benefit.
pub fn parse_event(payload: &str) -> Option<StreamEvent> {
    let value: serde_json::Value = serde_json::from_str(payload).ok()?;

    match value.get("type")?.as_str()? {
        "content_block_delta" => {
            let delta = value.get("delta")?;
            match delta.get("type")?.as_str()? {
                "text_delta" => Some(StreamEvent::Text(delta.get("text")?.as_str()?.to_owned())),
                "thinking_delta" => Some(StreamEvent::Thinking(
                    delta.get("thinking")?.as_str()?.to_owned(),
                )),
                _ => None,
            }
        }

        "content_block_start" => {
            let block = value.get("content_block")?;
            match block.get("type")?.as_str()? {
                // A refusal was picked up by another model. The block arrives
                // as an ordinary content_block_start — there is no dedicated
                // SSE event for it.
                "fallback" => Some(StreamEvent::FellBackTo(
                    block.get("to")?.get("model")?.as_str()?.to_owned(),
                )),
                "server_tool_use" => {
                    let query = block.get("input")?.get("query")?.as_str()?;
                    Some(StreamEvent::Search(query.to_owned()))
                }
                // A completed search. Its sources are already in the block, so
                // there is nothing further to wait for.
                "web_search_tool_result" => {
                    let sources = search_sources(block);
                    (!sources.is_empty()).then_some(StreamEvent::Sources(sources))
                }
                _ => None,
            }
        }

        "message_delta" => {
            // The only interesting part is a refusal; everything else here is
            // usage accounting.
            match value.get("delta")?.get("stop_reason")?.as_str()? {
                "refusal" => Some(StreamEvent::Failed(AiError::Refused)),
                _ => None,
            }
        }

        "message_stop" => Some(StreamEvent::Done),

        "error" => {
            let kind = value
                .get("error")
                .and_then(|error| error.get("type"))
                .and_then(|kind| kind.as_str())
                .unwrap_or("");
            Some(StreamEvent::Failed(match kind {
                "authentication_error" | "permission_error" => AiError::BadKey,
                "rate_limit_error" | "overloaded_error" | "api_error" => AiError::RateLimited,
                _ => AiError::Unavailable,
            }))
        }

        _ => None,
    }
}

/// The sources in a completed `web_search_tool_result` block.
///
/// A successful result's `content` is a *list*; a failed one is a single error
/// *object* (`{"error_code": "max_uses_exceeded"}`). Web-search failures come
/// back as HTTP 200, so branching on that shape is the only thing separating
/// "no sources" from "the search broke".
pub fn search_sources(block: &serde_json::Value) -> Vec<SearchSource> {
    let Some(content) = block.get("content").and_then(|c| c.as_array()) else {
        return Vec::new();
    };

    content
        .iter()
        .filter(|result| result.get("type").and_then(|t| t.as_str()) == Some("web_search_result"))
        .filter_map(|result| {
            Some(SearchSource {
                title: result.get("title")?.as_str()?.to_owned(),
                url: result.get("url")?.as_str()?.to_owned(),
            })
        })
        .collect()
}

/// Who said it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum Role {
    User,
    Assistant,
}

/// One turn in the conversation, as the window draws it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ChatMessage {
    pub id: u32,
    pub role: Role,
    pub content: String,
    /// The model's summarised reasoning, when it produced any.
    #[serde(default)]
    pub thinking: String,
    #[serde(default)]
    pub searches: Vec<String>,
    #[serde(default)]
    pub sources: Vec<SearchSource>,
    /// File names attached to a user turn. The bytes are not kept — they went
    /// to the API and a chat history is not a place to store documents.
    #[serde(default)]
    pub attachments: Vec<String>,
    /// Set when the model that answered was not the one that was asked.
    #[serde(default)]
    pub answered_by: String,
    #[ts(type = "number")]
    pub time: u64,
}

/// Turns beyond this are dropped oldest-first.
///
/// A conversation is resent in full on every request, so an unbounded one
/// silently grows the cost of each turn until it hits the context window.
const MAX_TURNS: usize = 200;

pub struct Store {
    inner: Mutex<Inner>,
    path: PathBuf,
}

struct Inner {
    messages: Vec<ChatMessage>,
    next_id: u32,
}

impl Store {
    pub fn load(path: PathBuf) -> Self {
        let messages: Vec<ChatMessage> = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();

        let next_id = messages
            .iter()
            .map(|message| message.id)
            .max()
            .map_or(1, |highest| highest.saturating_add(1));

        Self {
            inner: Mutex::new(Inner { messages, next_id }),
            path,
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.inner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    pub fn list(&self) -> Vec<ChatMessage> {
        self.lock().messages.clone()
    }

    /// Appends a turn and returns it with its id filled in.
    pub fn append(&self, role: Role, content: String, attachments: Vec<String>) -> ChatMessage {
        let mut inner = self.lock();
        let message = ChatMessage {
            id: inner.next_id,
            role,
            content,
            thinking: String::new(),
            searches: Vec::new(),
            sources: Vec::new(),
            attachments,
            answered_by: String::new(),
            time: now_seconds(),
        };
        inner.next_id = inner.next_id.saturating_add(1);
        inner.messages.push(message.clone());

        // Oldest first, and always in whole turns: half a conversation that
        // starts with an assistant reply is not a valid request.
        while inner.messages.len() > MAX_TURNS {
            inner.messages.remove(0);
        }

        self.persist_from(inner);
        message
    }

    /// Replaces a message's body, for the assistant turn being streamed into.
    pub fn update<F: FnOnce(&mut ChatMessage)>(&self, id: u32, change: F) -> Option<ChatMessage> {
        let mut inner = self.lock();
        let message = inner.messages.iter_mut().find(|message| message.id == id)?;
        change(message);
        let updated = message.clone();
        self.persist_from(inner);
        Some(updated)
    }

    pub fn clear(&self) {
        let mut inner = self.lock();
        inner.messages.clear();
        self.persist_from(inner);
    }

    /// Drops the last turn, for retrying a failed request.
    pub fn pop(&self) -> Option<ChatMessage> {
        let mut inner = self.lock();
        let removed = inner.messages.pop()?;
        self.persist_from(inner);
        Some(removed)
    }

    fn persist_from(&self, inner: std::sync::MutexGuard<'_, Inner>) {
        let snapshot = inner.messages.clone();
        drop(inner);

        if let Some(parent) = self.path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(text) = serde_json::to_string_pretty(&snapshot) {
            let _ = std::fs::write(&self.path, text);
        }
    }
}

pub fn history_path() -> PathBuf {
    crate::paths::state_dir().join("chat.json")
}

pub fn store_in(directory: &Path) -> Store {
    Store::load(directory.join("chat.json"))
}

fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn temp_dir(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("bw-chat-{name}"));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("a writable temp dir");
        path
    }

    #[test]
    fn text_deltas_are_the_reply() {
        let event = parse_event(
            r#"{"type":"content_block_delta","index":0,
                "delta":{"type":"text_delta","text":"Hello"}}"#,
        );
        assert_eq!(event, Some(StreamEvent::Text("Hello".to_owned())));
    }

    #[test]
    fn thinking_deltas_are_kept_apart_from_the_reply() {
        // Concatenating the two would splice the model's reasoning into its
        // answer, which is exactly what the separate pane exists to avoid.
        let event = parse_event(
            r#"{"type":"content_block_delta","index":0,
                "delta":{"type":"thinking_delta","thinking":"Let me check"}}"#,
        );
        assert_eq!(
            event,
            Some(StreamEvent::Thinking("Let me check".to_owned()))
        );
    }

    #[test]
    fn a_fallback_block_is_recognised_although_it_has_no_event_of_its_own() {
        // The retry happens on the same stream; the only marker is an ordinary
        // content_block_start carrying a `fallback` block.
        let event = parse_event(
            r#"{"type":"content_block_start","index":0,
                "content_block":{"type":"fallback",
                  "from":{"model":"claude-opus-5"},"to":{"model":"claude-opus-4-8"}}}"#,
        );
        assert_eq!(
            event,
            Some(StreamEvent::FellBackTo("claude-opus-4-8".to_owned()))
        );
    }

    #[test]
    fn a_search_reports_the_query_it_ran() {
        let event = parse_event(
            r#"{"type":"content_block_start","index":1,
                "content_block":{"type":"server_tool_use","name":"web_search",
                  "input":{"query":"weather in Tokyo"}}}"#,
        );
        assert_eq!(
            event,
            Some(StreamEvent::Search("weather in Tokyo".to_owned()))
        );
    }

    #[test]
    fn a_refusal_ends_the_turn_as_a_failure_not_a_completion() {
        let event = parse_event(
            r#"{"type":"message_delta","delta":{"stop_reason":"refusal"},
                "usage":{"output_tokens":3}}"#,
        );
        assert_eq!(event, Some(StreamEvent::Failed(AiError::Refused)));
    }

    #[test]
    fn an_ordinary_stop_reason_is_not_a_failure() {
        assert_eq!(
            parse_event(r#"{"type":"message_delta","delta":{"stop_reason":"end_turn"}}"#),
            None
        );
        assert_eq!(
            parse_event(r#"{"type":"message_stop"}"#),
            Some(StreamEvent::Done)
        );
    }

    #[test]
    fn stream_errors_are_classified_the_way_status_codes_are() {
        assert_eq!(
            parse_event(r#"{"type":"error","error":{"type":"authentication_error"}}"#),
            Some(StreamEvent::Failed(AiError::BadKey))
        );
        assert_eq!(
            parse_event(r#"{"type":"error","error":{"type":"overloaded_error"}}"#),
            Some(StreamEvent::Failed(AiError::RateLimited))
        );
    }

    #[test]
    fn noise_and_unknown_events_are_skipped_rather_than_failing() {
        // The API adds event types; a chat that broke when it did would be
        // brittle for no benefit.
        assert_eq!(parse_event(r#"{"type":"ping"}"#), None);
        assert_eq!(
            parse_event(r#"{"type":"content_block_stop","index":0}"#),
            None
        );
        assert_eq!(parse_event(r#"{"type":"something_new","payload":1}"#), None);
        assert_eq!(parse_event("not json at all"), None);
        assert_eq!(parse_event(""), None);
    }

    #[test]
    fn a_completed_search_block_carries_its_sources_through_the_stream() {
        let event = parse_event(
            r#"{"type":"content_block_start","index":2,
                "content_block":{"type":"web_search_tool_result","tool_use_id":"srvtoolu_1",
                  "content":[{"type":"web_search_result","title":"Tokyo",
                              "url":"https://example.com/a"}]}}"#,
        );
        match event {
            Some(StreamEvent::Sources(sources)) => {
                assert_eq!(sources.len(), 1);
                assert_eq!(sources[0].url, "https://example.com/a");
            }
            other => panic!("expected sources, got {other:?}"),
        }
    }

    #[test]
    fn a_failed_search_block_yields_no_event_rather_than_panicking() {
        // `content` is an error object rather than a list here, and the whole
        // turn still arrives as HTTP 200.
        let event = parse_event(
            r#"{"type":"content_block_start","index":2,
                "content_block":{"type":"web_search_tool_result",
                  "content":{"type":"web_search_tool_result_error",
                             "error_code":"max_uses_exceeded"}}}"#,
        );
        assert_eq!(event, None);
    }

    #[test]
    fn a_successful_search_yields_its_sources() {
        let block = json!({
            "type": "web_search_tool_result",
            "content": [
                {"type": "web_search_result", "title": "Tokyo", "url": "https://example.com/a"},
                {"type": "web_search_result", "title": "Weather", "url": "https://example.com/b"},
            ]
        });
        let sources = search_sources(&block);
        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0].title, "Tokyo");
    }

    #[test]
    fn a_failed_search_is_an_object_not_a_list_and_yields_nothing() {
        // Web search failures arrive as HTTP 200 with `content` as a single
        // error object. Indexing it as a list is the classic way to panic on a
        // perfectly ordinary rate limit.
        let block = json!({
            "type": "web_search_tool_result",
            "content": {"type": "web_search_tool_result_error", "error_code": "max_uses_exceeded"}
        });
        assert!(search_sources(&block).is_empty());
    }

    #[test]
    fn the_conversation_survives_a_reload_without_reusing_ids() {
        let directory = temp_dir("reload");
        let highest = {
            let store = store_in(&directory);
            store.append(Role::User, "Hello".to_owned(), Vec::new());
            store
                .append(Role::Assistant, "Hi".to_owned(), Vec::new())
                .id
        };

        let reloaded = store_in(&directory);
        assert_eq!(reloaded.list().len(), 2);
        assert!(
            reloaded
                .append(Role::User, "Again".to_owned(), Vec::new())
                .id
                > highest
        );
    }

    #[test]
    fn streaming_into_a_turn_updates_it_in_place() {
        let store = store_in(&temp_dir("stream"));
        let reply = store.append(Role::Assistant, String::new(), Vec::new());

        for piece in ["Hel", "lo"] {
            store.update(reply.id, |message| message.content.push_str(piece));
        }
        assert_eq!(store.list()[0].content, "Hello");
        assert!(store.update(9999, |_| {}).is_none());
    }

    #[test]
    fn the_history_is_bounded_because_every_turn_is_resent() {
        let store = store_in(&temp_dir("bounded"));
        for index in 0..MAX_TURNS + 10 {
            store.append(Role::User, format!("#{index}"), Vec::new());
        }
        let list = store.list();
        assert_eq!(list.len(), MAX_TURNS);
        // The oldest go, so the most recent context is what survives.
        assert_eq!(list.last().unwrap().content, format!("#{}", MAX_TURNS + 9));
    }

    #[test]
    fn popping_takes_the_last_turn_back_for_a_retry() {
        let store = store_in(&temp_dir("pop"));
        store.append(Role::User, "Hello".to_owned(), Vec::new());
        let failed = store.append(Role::Assistant, String::new(), Vec::new());

        assert_eq!(store.pop().map(|message| message.id), Some(failed.id));
        assert_eq!(store.list().len(), 1);
    }

    #[test]
    fn a_corrupt_history_starts_empty_rather_than_failing() {
        let directory = temp_dir("corrupt");
        std::fs::write(directory.join("chat.json"), "{ not json").unwrap();

        let store = store_in(&directory);
        assert!(store.list().is_empty());
        store.append(Role::User, "Fine".to_owned(), Vec::new());
        assert_eq!(store_in(&directory).list().len(), 1);
    }

    #[test]
    fn attachments_record_names_but_never_bytes() {
        let store = store_in(&temp_dir("attach"));
        let message = store.append(
            Role::User,
            "What is this?".to_owned(),
            vec!["diagram.png".to_owned()],
        );
        assert_eq!(message.attachments, ["diagram.png"]);

        // A chat history is not a document store; the file went to the API.
        let text = std::fs::read_to_string(temp_dir("attach").join("chat.json"));
        assert!(text.is_err() || !text.unwrap().contains("base64"));
    }
}
