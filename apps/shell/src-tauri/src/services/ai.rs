//! Talking to the Anthropic API.
//!
//! Rust has no official Anthropic SDK, so this is the Messages API over plain
//! HTTP with the `reqwest` already in the tree. Only one caller exists today —
//! the sidebar's translator — but the shape is a conversation rather than a
//! single string, because the chat that comes later sends the same request
//! with more messages in it.
//!
//! The key is never written to `config.json`. It goes to the Windows
//! credential manager through `keyring`, the same store the online wallpaper
//! providers use, so a config file someone pastes into an issue carries no
//! secret.

use base64::Engine as _;
use bw_core::ai::{AiError, AiMessage, ApiResponse};
use bw_core::chat::{self, ChatMessage, Role, StreamEvent};
use bw_core::Config;
use futures_util::StreamExt as _;

const ENDPOINT: &str = "https://api.anthropic.com/v1/messages";

/// The API version header. Pinned rather than tracking latest: a version bump
/// can change the response shape, and that should be a deliberate edit here.
const API_VERSION: &str = "2023-06-01";

/// Lets the API re-run a request another model declined, instead of handing
/// the refusal back. `"default"` routes by refusal category rather than
/// pinning a substitute, so there is no migration owed when a pinned model is
/// retired.
const FALLBACK_BETA: &str = "server-side-fallback-2026-07-01";

/// The web-search tool. The dated variant matters: it is the one with dynamic
/// filtering, and it runs code execution internally — so `code_execution`
/// must *not* also be declared, or the model sees two environments.
const WEB_SEARCH_TOOL: &str = "web_search_20260209";

/// Attachments are read whole into memory and base64'd into the request, so
/// this is both a request-size guard and a memory one. The API's own limit is
/// 32 MB for the whole request.
const MAX_ATTACHMENT_BYTES: u64 = 12 * 1024 * 1024;

/// Where the key lives, alongside the wallpaper providers' keys.
const KEYRING_SERVICE: &str = "beautiful-wallpaper";
const KEYRING_ACCOUNT: &str = "anthropic";

/// Whether a key has been configured, without revealing it.
///
/// The sidebar uses this to decide between showing the translator and showing
/// a pointer at the settings — a first run is not an error state.
pub fn has_key() -> bool {
    read_key().is_some()
}

fn read_key() -> Option<String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT).ok()?;
    entry
        .get_password()
        .ok()
        .map(|key| key.trim().to_owned())
        .filter(|key| !key.is_empty())
}

/// Stores the key, or clears it when given an empty string.
pub fn set_key(key: &str) -> Result<(), String> {
    let entry =
        keyring::Entry::new(KEYRING_SERVICE, KEYRING_ACCOUNT).map_err(|error| error.to_string())?;

    if key.trim().is_empty() {
        // Deleting a key that was never set is not an error worth reporting.
        let _ = entry.delete_credential();
        return Ok(());
    }
    entry
        .set_password(key.trim())
        .map_err(|error| error.to_string())
}

/// Sends a conversation and returns the reply's text.
pub async fn ask(
    config: &Config,
    system: Option<String>,
    messages: Vec<AiMessage>,
) -> Result<String, AiError> {
    let Some(key) = read_key() else {
        return Err(AiError::NoKey);
    };

    let mut body = serde_json::json!({
        "model": config.ai.model,
        "max_tokens": config.ai.max_tokens,
        "messages": messages,
    });
    if let Some(system) = system {
        body["system"] = serde_json::Value::String(system);
    }

    let response = reqwest::Client::new()
        .post(ENDPOINT)
        .header("x-api-key", key)
        .header("anthropic-version", API_VERSION)
        .json(&body)
        .send()
        .await
        .map_err(|error| {
            tracing::debug!(%error, "the API could not be reached");
            AiError::Unavailable
        })?;

    let status = response.status();
    if !status.is_success() {
        // The body carries a reason, which is worth logging but not worth
        // showing: it is English prose from a service the user did not choose
        // to read. The classified error is what the UI acts on.
        let detail = response.text().await.unwrap_or_default();
        tracing::warn!(status = status.as_u16(), %detail, "the API refused a request");
        return Err(AiError::from_status(status.as_u16()));
    }

    let parsed: ApiResponse = response.json().await.map_err(|error| {
        tracing::warn!(%error, "the API returned something unreadable");
        AiError::Unavailable
    })?;

    parsed.text()
}

/// Translates one piece of text.
pub async fn translate(
    config: &Config,
    text: &str,
    from: &str,
    to: &str,
) -> Result<String, AiError> {
    // Nothing to translate is not a failure, and sending it would spend a
    // request to be told the same.
    if text.trim().is_empty() {
        return Ok(String::new());
    }

    let system = bw_core::ai::translation_prompt(from, to);
    ask(config, Some(system), vec![AiMessage::user(text)]).await
}

/// A file to send alongside a message.
pub struct Attachment {
    pub name: String,
    /// `image` or `document` — the API's block type must match the file.
    pub kind: &'static str,
    pub media_type: String,
    pub data: String,
}

/// Reads a file into the block shape the API wants.
///
/// Images and PDFs take different block types, and sending one as the other is
/// rejected — so the type is decided here from the extension rather than
/// guessed at the call site.
pub fn read_attachment(path: &std::path::Path) -> Result<Attachment, String> {
    let metadata = std::fs::metadata(path).map_err(|error| format!("{error}"))?;
    if metadata.len() > MAX_ATTACHMENT_BYTES {
        return Err(format!(
            "{} is too large to attach ({} MB); the limit is {} MB",
            path.display(),
            metadata.len() / 1024 / 1024,
            MAX_ATTACHMENT_BYTES / 1024 / 1024
        ));
    }

    let extension = path
        .extension()
        .map(|extension| extension.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    let (kind, media_type) = match extension.as_str() {
        "png" => ("image", "image/png"),
        "jpg" | "jpeg" => ("image", "image/jpeg"),
        "gif" => ("image", "image/gif"),
        "webp" => ("image", "image/webp"),
        "pdf" => ("document", "application/pdf"),
        other => {
            return Err(format!(
                "{other} files cannot be attached; images and PDFs can"
            ))
        }
    };

    let bytes = std::fs::read(path).map_err(|error| format!("{error}"))?;
    Ok(Attachment {
        name: path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default(),
        kind,
        media_type: media_type.to_owned(),
        // No line breaks: the API rejects a wrapped base64 payload.
        data: base64::engine::general_purpose::STANDARD.encode(bytes),
    })
}

/// Builds the `messages` array from the stored conversation.
///
/// Attachments are only ever on the newest user turn: the bytes are not kept
/// in the history, so replaying an older turn's files is not possible — and
/// re-uploading them every turn would be expensive even if it were.
fn build_messages(history: &[ChatMessage], attachments: &[Attachment]) -> Vec<serde_json::Value> {
    let last = history.len().saturating_sub(1);

    history
        .iter()
        .enumerate()
        .filter(|(_, message)| !message.content.trim().is_empty())
        .map(|(index, message)| {
            let role = match message.role {
                Role::User => "user",
                Role::Assistant => "assistant",
            };

            if index != last || attachments.is_empty() {
                return serde_json::json!({ "role": role, "content": message.content });
            }

            // Documents and images go before the text, which is what the API
            // asks for and what the model reads best.
            let mut blocks: Vec<serde_json::Value> = attachments
                .iter()
                .map(|attachment| {
                    serde_json::json!({
                        "type": attachment.kind,
                        "source": {
                            "type": "base64",
                            "media_type": attachment.media_type,
                            "data": attachment.data,
                        }
                    })
                })
                .collect();
            blocks.push(serde_json::json!({ "type": "text", "text": message.content }));

            serde_json::json!({ "role": role, "content": blocks })
        })
        .collect()
}

/// Streams a reply, calling `on_event` for each thing worth showing.
///
/// Streaming rather than waiting for the whole reply: a long answer takes
/// minutes, and an empty pane for that long reads as a hang. It also keeps the
/// request under the HTTP timeouts a large `max_tokens` would otherwise hit.
pub async fn stream(
    config: &Config,
    history: &[ChatMessage],
    attachments: &[Attachment],
    on_event: impl Fn(StreamEvent),
) {
    let Some(key) = read_key() else {
        on_event(StreamEvent::Failed(AiError::NoKey));
        return;
    };

    let mut body = serde_json::json!({
        "model": config.ai.model,
        "max_tokens": config.ai.max_tokens,
        "messages": build_messages(history, attachments),
        "stream": true,
        // Adaptive thinking is the current shape; `budget_tokens` is rejected
        // on this model. `summarized` is needed explicitly — the default is
        // `omitted`, which streams thinking blocks with no text in them.
        "thinking": { "type": "adaptive", "display": "summarized" },
        // Routes a refusal to whichever model is recommended for its category
        // rather than pinning one here.
        "fallbacks": "default",
    });

    if config.ai.web_search {
        body["tools"] = serde_json::json!([
            { "type": WEB_SEARCH_TOOL, "name": "web_search", "max_uses": config.ai.max_searches }
        ]);
    }

    let response = reqwest::Client::new()
        .post(ENDPOINT)
        .header("x-api-key", key)
        .header("anthropic-version", API_VERSION)
        .header("anthropic-beta", FALLBACK_BETA)
        .json(&body)
        .send()
        .await;

    let response = match response {
        Ok(response) => response,
        Err(error) => {
            tracing::debug!(%error, "the API could not be reached");
            on_event(StreamEvent::Failed(AiError::Unavailable));
            return;
        }
    };

    let status = response.status();
    if !status.is_success() {
        let detail = response.text().await.unwrap_or_default();
        tracing::warn!(status = status.as_u16(), %detail, "the API refused a request");
        on_event(StreamEvent::Failed(AiError::from_status(status.as_u16())));
        return;
    }

    let mut stream = response.bytes_stream();
    // SSE frames are split on blank lines and arrive across arbitrary chunk
    // boundaries, so a partial frame has to survive until the rest turns up.
    let mut buffer = String::new();
    let mut finished = false;

    while let Some(chunk) = stream.next().await {
        let Ok(chunk) = chunk else {
            on_event(StreamEvent::Failed(AiError::Unavailable));
            return;
        };
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(split) = buffer.find("\n\n") {
            let frame = buffer[..split].to_owned();
            buffer.drain(..split + 2);

            for line in frame.lines() {
                let Some(payload) = line.strip_prefix("data:") else {
                    continue;
                };
                let Some(event) = chat::parse_event(payload.trim()) else {
                    continue;
                };
                if matches!(event, StreamEvent::Done | StreamEvent::Failed(_)) {
                    finished = true;
                }
                on_event(event);
            }
        }
    }

    // A stream that stops without saying so leaves the reply looking like it
    // is still arriving. Close it out rather than spinning forever.
    if !finished {
        on_event(StreamEvent::Done);
    }
}
