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

use bw_core::ai::{AiError, AiMessage, ApiResponse};
use bw_core::Config;

const ENDPOINT: &str = "https://api.anthropic.com/v1/messages";

/// The API version header. Pinned rather than tracking latest: a version bump
/// can change the response shape, and that should be a deliberate edit here.
const API_VERSION: &str = "2023-06-01";

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
