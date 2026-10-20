//! Talking to the chat services: Anthropic, Gemini and OpenAI.
//!
//! None of the three has an official Rust SDK, so each is its HTTP API over
//! the `reqwest` already in the tree: Anthropic's Messages API, Gemini's
//! `generateContent`, OpenAI's Responses API. The chat and the translator go
//! to whichever `ai.provider` names; what differs is only the request's shape
//! and the stream's, and the stream's is parsed in `bw_core::chat`.
//!
//! The keys are never written to `config.json`. They go to the Windows
//! credential manager through `keyring`, the same store the online wallpaper
//! providers use, so a config file someone pastes into an issue carries no
//! secret.

use base64::Engine as _;
use bw_core::ai::{AiError, AiMessage, AiModel, ApiResponse, Provider};
use bw_core::chat::{self, ChatMessage, Role, StreamEvent};
use bw_core::Config;
use futures_util::StreamExt as _;
use serde_json::{json, Value};

const ENDPOINT: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_MODELS: &str = "https://api.anthropic.com/v1/models";
const GEMINI: &str = "https://generativelanguage.googleapis.com/v1beta";
const OPENAI: &str = "https://api.openai.com/v1";

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

/// Where the keys live, alongside the wallpaper providers' keys. Each
/// service's key is under its own name, Anthropic's where it always was.
const KEYRING_SERVICE: &str = "beautiful-wallpaper";

/// The service `ai.provider` names.
pub fn provider(config: &Config) -> Provider {
    Provider::parse(&config.ai.provider)
}

/// Whether a key has been configured for a service, without revealing it.
///
/// The sidebar uses this to decide between showing the chat and showing a
/// pointer at the settings — a first run is not an error state.
pub fn has_key(provider: Provider) -> bool {
    read_key(provider).is_some()
}

fn read_key(provider: Provider) -> Option<String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, provider.as_str()).ok()?;
    entry
        .get_password()
        .ok()
        .map(|key| key.trim().to_owned())
        .filter(|key| !key.is_empty())
}

/// Stores a service's key, or clears it when given an empty string.
pub fn set_key(provider: Provider, key: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, provider.as_str())
        .map_err(|error| error.to_string())?;

    if key.trim().is_empty() {
        // Deleting a key that was never set is not an error worth reporting.
        let _ = entry.delete_credential();
        return Ok(());
    }
    entry
        .set_password(key.trim())
        .map_err(|error| error.to_string())
}

/// A request to a service with its key on it.
fn post(provider: Provider, key: &str, model: &str, streaming: bool) -> reqwest::RequestBuilder {
    let client = reqwest::Client::new();
    match provider {
        Provider::Anthropic => client
            .post(ENDPOINT)
            .header("x-api-key", key)
            .header("anthropic-version", API_VERSION),
        Provider::Gemini => {
            let method = if streaming {
                "streamGenerateContent?alt=sse"
            } else {
                "generateContent"
            };
            // The list names models `models/…`; either spelling is accepted.
            let model = model.trim_start_matches("models/");
            client
                .post(format!("{GEMINI}/models/{model}:{method}"))
                .header("x-goog-api-key", key)
        }
        Provider::OpenAi => client.post(format!("{OPENAI}/responses")).bearer_auth(key),
    }
}

/// Sends a request, turning a refusal into the outcome the UI acts on.
async fn send(request: reqwest::RequestBuilder) -> Result<reqwest::Response, AiError> {
    let response = request.send().await.map_err(|error| {
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
        return Err(AiError::from_response(status.as_u16(), &detail));
    }
    Ok(response)
}

/// The request body, in the shape the service wants. `messages` are already
/// in that shape; `chat` adds what the chat asks for and the translator does
/// not — streaming, the web search, the model's reasoning.
fn body(
    provider: Provider,
    config: &Config,
    system: Option<String>,
    messages: Vec<Value>,
    chat: bool,
) -> Value {
    let model = provider.model(&config.ai);
    let max_tokens = config.ai.max_tokens;

    match provider {
        Provider::Anthropic => {
            let mut body = json!({
                "model": model,
                "max_tokens": max_tokens,
                "messages": messages,
            });
            if let Some(system) = system {
                body["system"] = Value::String(system);
            }
            if chat {
                body["stream"] = json!(true);
                // Adaptive thinking is the current shape; `budget_tokens` is
                // rejected on this model. `summarized` is needed explicitly —
                // the default is `omitted`, which streams thinking blocks with
                // no text in them.
                body["thinking"] = json!({ "type": "adaptive", "display": "summarized" });
                // Routes a refusal to whichever model is recommended for its
                // category rather than pinning one here.
                body["fallbacks"] = json!("default");
                if config.ai.web_search {
                    body["tools"] = json!([{
                        "type": WEB_SEARCH_TOOL,
                        "name": "web_search",
                        "max_uses": config.ai.max_searches,
                    }]);
                }
            }
            body
        }

        Provider::Gemini => {
            let mut generation = json!({ "maxOutputTokens": max_tokens });
            let mut body = json!({ "contents": messages });
            if let Some(system) = system {
                body["systemInstruction"] = json!({ "parts": [{ "text": system }] });
            }
            if chat {
                if config.ai.show_thinking {
                    generation["thinkingConfig"] = json!({ "includeThoughts": true });
                }
                // ponytail: Gemini's search has no per-turn cap, so
                // `ai.maxSearches` does not apply to it.
                if config.ai.web_search {
                    body["tools"] = json!([{ "google_search": {} }]);
                }
            }
            body["generationConfig"] = generation;
            body
        }

        Provider::OpenAi => {
            let mut body = json!({
                "model": model,
                "input": messages,
                "max_output_tokens": max_tokens,
            });
            if let Some(system) = system {
                body["instructions"] = Value::String(system);
            }
            if chat {
                body["stream"] = json!(true);
                // ponytail: no reasoning summaries — OpenAI only gives them to
                // organisations it has verified, and asking without that fails
                // the whole request. The search has no per-turn cap either.
                if config.ai.web_search {
                    body["tools"] = json!([{ "type": "web_search" }]);
                }
            }
            body
        }
    }
}

/// Sends a conversation and returns the reply's text.
pub async fn ask(
    config: &Config,
    system: Option<String>,
    messages: Vec<AiMessage>,
) -> Result<String, AiError> {
    let provider = provider(config);
    let Some(key) = read_key(provider) else {
        return Err(AiError::NoKey);
    };

    let messages = messages
        .into_iter()
        .map(|message| match provider {
            Provider::Gemini => {
                let role = if message.role == "assistant" {
                    "model"
                } else {
                    "user"
                };
                json!({ "role": role, "parts": [{ "text": message.content }] })
            }
            _ => json!({ "role": message.role, "content": message.content }),
        })
        .collect();

    let request = post(provider, &key, provider.model(&config.ai), false)
        .json(&body(provider, config, system, messages, false));
    let reply: Value = send(request).await?.json().await.map_err(|error| {
        tracing::warn!(%error, "the API returned something unreadable");
        AiError::Unavailable
    })?;

    match provider {
        Provider::Anthropic => serde_json::from_value::<ApiResponse>(reply)
            .map_err(|_| AiError::Unavailable)?
            .text(),
        Provider::Gemini => bw_core::ai::gemini_reply(&reply),
        Provider::OpenAi => bw_core::ai::openai_reply(&reply),
    }
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

/// The models a service offers, for the settings to choose from — so a model
/// released tomorrow is in the list without a new version of the shell.
pub async fn list_models(provider: Provider) -> Result<Vec<AiModel>, AiError> {
    let Some(key) = read_key(provider) else {
        return Err(AiError::NoKey);
    };

    let client = reqwest::Client::new();
    // Each list is paged; one page as large as each allows holds them all.
    let request = match provider {
        Provider::Anthropic => client
            .get(format!("{ANTHROPIC_MODELS}?limit=1000"))
            .header("x-api-key", key.as_str())
            .header("anthropic-version", API_VERSION),
        Provider::Gemini => client
            .get(format!("{GEMINI}/models?pageSize=1000"))
            .header("x-goog-api-key", key.as_str()),
        Provider::OpenAi => client
            .get(format!("{OPENAI}/models"))
            .bearer_auth(key.as_str()),
    };

    let list: Value = send(request)
        .await?
        .json()
        .await
        .map_err(|_| AiError::Unavailable)?;
    Ok(bw_core::ai::models_from(provider, &list))
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

/// Builds the conversation, in the service's shape, from the stored one.
///
/// Attachments are only ever on the newest user turn: the bytes are not kept
/// in the history, so replaying an older turn's files is not possible — and
/// re-uploading them every turn would be expensive even if it were.
fn build_messages(
    provider: Provider,
    history: &[ChatMessage],
    attachments: &[Attachment],
) -> Vec<Value> {
    let last = history.len().saturating_sub(1);

    history
        .iter()
        .enumerate()
        .filter(|(_, message)| !message.content.trim().is_empty())
        .map(|(index, message)| {
            let role = match (message.role, provider) {
                (Role::User, _) => "user",
                (Role::Assistant, Provider::Gemini) => "model",
                (Role::Assistant, _) => "assistant",
            };
            let files: &[Attachment] = if index == last { attachments } else { &[] };

            // Files go before the text, which is what each API asks for and
            // what the model reads best.
            match provider {
                Provider::Gemini => {
                    let mut parts: Vec<Value> = files
                        .iter()
                        .map(|file| {
                            json!({ "inlineData": { "mimeType": file.media_type, "data": file.data } })
                        })
                        .collect();
                    parts.push(json!({ "text": message.content }));
                    json!({ "role": role, "parts": parts })
                }
                _ if files.is_empty() => json!({ "role": role, "content": message.content }),
                Provider::Anthropic => {
                    let mut blocks: Vec<Value> = files
                        .iter()
                        .map(|file| {
                            json!({
                                "type": file.kind,
                                "source": {
                                    "type": "base64",
                                    "media_type": file.media_type,
                                    "data": file.data,
                                }
                            })
                        })
                        .collect();
                    blocks.push(json!({ "type": "text", "text": message.content }));
                    json!({ "role": role, "content": blocks })
                }
                Provider::OpenAi => {
                    let mut blocks: Vec<Value> = files
                        .iter()
                        .map(|file| {
                            let url = format!("data:{};base64,{}", file.media_type, file.data);
                            if file.kind == "image" {
                                json!({ "type": "input_image", "image_url": url })
                            } else {
                                json!({ "type": "input_file", "filename": file.name, "file_data": url })
                            }
                        })
                        .collect();
                    blocks.push(json!({ "type": "input_text", "text": message.content }));
                    json!({ "role": role, "content": blocks })
                }
            }
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
    let provider = provider(config);
    let Some(key) = read_key(provider) else {
        on_event(StreamEvent::Failed(AiError::NoKey));
        return;
    };

    let messages = build_messages(provider, history, attachments);
    let mut body = body(provider, config, None, messages, true);
    let request = |body: &Value| {
        let request = post(provider, &key, provider.model(&config.ai), true).json(body);
        if provider == Provider::Anthropic {
            request.header("anthropic-beta", FALLBACK_BETA)
        } else {
            request
        }
    };

    let mut response = send(request(&body)).await;
    // A free Gemini key has no quota for Google Search, and says so with the
    // same 429 as any other limit — so with the search on, every question
    // failed. Asked again without it, the question is still answered, only
    // not from the web.
    let without_search = provider == Provider::Gemini
        && matches!(response, Err(AiError::RateLimited))
        && body
            .as_object_mut()
            .and_then(|fields| fields.remove("tools"))
            .is_some();
    if without_search {
        tracing::info!("asking Gemini again without its search");
        response = send(request(&body)).await;
    }

    let response = match response {
        Ok(response) => response,
        Err(error) => {
            on_event(StreamEvent::Failed(error));
            return;
        }
    };

    let mut stream = response.bytes_stream();
    // SSE frames are split on blank lines and arrive across arbitrary chunk
    // boundaries, so a partial frame has to survive until the rest turns up.
    // Kept as bytes until a frame is whole: a character split across two
    // chunks — any Japanese one can be — decoded half at a time comes out as
    // two replacement characters.
    let mut buffer: Vec<u8> = Vec::new();
    let mut finished = false;

    while let Some(chunk) = stream.next().await {
        let Ok(chunk) = chunk else {
            on_event(StreamEvent::Failed(AiError::Unavailable));
            return;
        };
        // Gemini ends its lines with CRLF. A payload never holds a raw CR —
        // JSON escapes it — so dropping them all is safe.
        buffer.extend(chunk.iter().filter(|byte| **byte != b'\r'));

        while let Some(split) = buffer.windows(2).position(|pair| pair == b"\n\n") {
            let frame = String::from_utf8_lossy(&buffer[..split]).into_owned();
            buffer.drain(..split + 2);

            for line in frame.lines() {
                let Some(payload) = line.strip_prefix("data:") else {
                    continue;
                };
                let payload = payload.trim();
                let events: Vec<StreamEvent> = match provider {
                    Provider::Anthropic => chat::parse_event(payload).into_iter().collect(),
                    Provider::Gemini => chat::parse_gemini(payload),
                    Provider::OpenAi => chat::parse_openai(payload).into_iter().collect(),
                };
                for event in events {
                    if matches!(event, StreamEvent::Done | StreamEvent::Failed(_)) {
                        finished = true;
                    }
                    on_event(event);
                }
            }
        }
    }

    // A stream that stops without saying so leaves the reply looking like it
    // is still arriving. Close it out rather than spinning forever.
    if !finished {
        on_event(StreamEvent::Done);
    }
}
