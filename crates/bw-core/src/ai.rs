//! Shapes and failure classification for the Anthropic API.
//!
//! The request itself lives in the shell crate — it needs `reqwest` and the
//! credential store. What lives here is everything that decides *what the user
//! is told*, because that is the part worth testing: a translator that says
//! "something went wrong" for a missing key, an expired key and a rate limit
//! alike leaves the user with nothing to act on.
//!
//! Rust has no official Anthropic SDK, so the shell calls the Messages API
//! over plain HTTP. These types are the small slice of that wire format the
//! shell actually reads.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Why a request could not be completed, in terms the UI can act on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub enum AiError {
    /// No key has been configured. The UI points at the settings rather than
    /// showing an error — this is the first-run state, not a failure.
    NoKey,
    /// The key was rejected. Only the user can fix this.
    BadKey,
    /// Rate limited or out of credit. Worth retrying later, unchanged.
    RateLimited,
    /// The API is unreachable, or it answered with something unusable.
    Unavailable,
    /// The model declined to answer. Rare for a translation, but it is a
    /// distinct outcome from a transport failure and must not be reported as
    /// one.
    Refused,
}

impl AiError {
    /// Maps an HTTP status onto an outcome.
    ///
    /// 401 and 403 are the user's problem; 429 and 5xx are worth retrying;
    /// everything else is lumped into unavailable because there is nothing
    /// more useful to say about it.
    pub fn from_status(status: u16) -> Self {
        match status {
            401 | 403 => Self::BadKey,
            429 | 500..=599 => Self::RateLimited,
            _ => Self::Unavailable,
        }
    }

    /// Whether trying the identical request again could plausibly work.
    pub fn is_retryable(self) -> bool {
        matches!(self, Self::RateLimited | Self::Unavailable)
    }
}

/// A message in a conversation. One request's worth today; the chat that comes
/// later sends a list of these.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct AiMessage {
    /// `user` or `assistant` — the only two the Messages API accepts here.
    pub role: String,
    pub content: String,
}

impl AiMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_owned(),
            content: content.into(),
        }
    }
}

/// The subset of a Messages API response the shell reads.
#[derive(Debug, Clone, Deserialize)]
pub struct ApiResponse {
    #[serde(default)]
    pub content: Vec<ApiBlock>,
    #[serde(default)]
    pub stop_reason: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiBlock {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub text: String,
}

impl ApiResponse {
    /// The reply's text, or why there is none.
    ///
    /// A response carries blocks of several kinds — thinking blocks among them
    /// — and only the text ones are the answer. Taking `content[0]` blindly
    /// would return an empty string the moment thinking is switched on.
    pub fn text(&self) -> Result<String, AiError> {
        if self.stop_reason.as_deref() == Some("refusal") {
            return Err(AiError::Refused);
        }

        let text: String = self
            .content
            .iter()
            .filter(|block| block.kind == "text")
            .map(|block| block.text.as_str())
            .collect::<Vec<_>>()
            .join("");

        if text.trim().is_empty() {
            return Err(AiError::Unavailable);
        }
        Ok(text)
    }
}

/// The instruction that turns the chat endpoint into a translator.
///
/// Explicit about returning nothing but the translation: a model asked to
/// translate will otherwise often add "Here is the translation:", which would
/// end up pasted into whatever the user is writing.
pub fn translation_prompt(from: &str, to: &str) -> String {
    let source = if from == "auto" {
        "Detect the source language.".to_owned()
    } else {
        format!("The source language is {from}.")
    };

    format!(
        "You are a translation engine. {source} Translate the user's text into \
         {to}. Reply with the translation and nothing else — no preamble, no \
         quotes around it, no notes, no explanation. Preserve the original \
         line breaks and any markup. If the text is already in {to}, return it \
         unchanged."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rejected_key_is_told_apart_from_a_rate_limit() {
        // The whole point of the enum: these need different messages.
        assert_eq!(AiError::from_status(401), AiError::BadKey);
        assert_eq!(AiError::from_status(403), AiError::BadKey);
        assert_eq!(AiError::from_status(429), AiError::RateLimited);
        assert_eq!(AiError::from_status(503), AiError::RateLimited);
        assert_eq!(AiError::from_status(400), AiError::Unavailable);
    }

    #[test]
    fn only_the_transient_failures_are_worth_retrying() {
        assert!(AiError::RateLimited.is_retryable());
        assert!(AiError::Unavailable.is_retryable());
        // Retrying either of these forever would just burn requests.
        assert!(!AiError::BadKey.is_retryable());
        assert!(!AiError::NoKey.is_retryable());
        assert!(!AiError::Refused.is_retryable());
    }

    #[test]
    fn the_reply_is_the_text_blocks_only() {
        // A response with thinking enabled leads with a non-text block; taking
        // the first block would return an empty string.
        let response: ApiResponse = serde_json::from_str(
            r#"{"content":[{"type":"thinking","thinking":"..."},
                           {"type":"text","text":"Bonjour"}]}"#,
        )
        .unwrap();
        assert_eq!(response.text().unwrap(), "Bonjour");
    }

    #[test]
    fn several_text_blocks_are_joined_rather_than_truncated() {
        let response: ApiResponse = serde_json::from_str(
            r#"{"content":[{"type":"text","text":"Bon"},{"type":"text","text":"jour"}]}"#,
        )
        .unwrap();
        assert_eq!(response.text().unwrap(), "Bonjour");
    }

    #[test]
    fn a_refusal_is_not_reported_as_a_network_failure() {
        let response: ApiResponse =
            serde_json::from_str(r#"{"content":[],"stop_reason":"refusal"}"#).unwrap();
        assert_eq!(response.text(), Err(AiError::Refused));
    }

    #[test]
    fn an_empty_reply_is_a_failure_rather_than_an_empty_translation() {
        // Silently replacing the user's text with nothing would look like the
        // translator had eaten it.
        let response: ApiResponse =
            serde_json::from_str(r#"{"content":[{"type":"text","text":"   "}]}"#).unwrap();
        assert_eq!(response.text(), Err(AiError::Unavailable));
    }

    #[test]
    fn an_unexpected_response_shape_does_not_fail_to_parse() {
        // Fields the shell does not read must not make the whole response
        // unusable when the API adds one.
        let response: ApiResponse = serde_json::from_str(
            r#"{"id":"msg_1","model":"claude-opus-5","usage":{"input_tokens":5},
                "content":[{"type":"text","text":"ok"}]}"#,
        )
        .unwrap();
        assert_eq!(response.text().unwrap(), "ok");
    }

    #[test]
    fn the_prompt_names_both_languages_and_forbids_a_preamble() {
        let prompt = translation_prompt("ja", "en");
        assert!(prompt.contains("source language is ja"));
        assert!(prompt.contains("into en"));
        assert!(prompt.contains("nothing else"));

        // `auto` asks for detection rather than naming a language called auto.
        let detected = translation_prompt("auto", "fr");
        assert!(detected.contains("Detect the source language"));
        assert!(!detected.contains("source language is auto"));
    }
}
