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

    /// As [`Self::from_status`], reading the body as well: Gemini answers a
    /// wrong key with a 400 that only its body tells apart from a bad request.
    pub fn from_response(status: u16, body: &str) -> Self {
        if body.contains("API_KEY_INVALID") {
            return Self::BadKey;
        }
        Self::from_status(status)
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

/// The services the chat and the translator can talk to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Provider {
    Anthropic,
    Gemini,
    OpenAi,
}

impl Provider {
    /// Anything unrecognised is Anthropic: a typo in a hand-edited file should
    /// leave the chat talking to the default service, not to none.
    pub fn parse(name: &str) -> Self {
        match name {
            "gemini" => Self::Gemini,
            "openai" => Self::OpenAi,
            _ => Self::Anthropic,
        }
    }

    /// The name in `ai.provider`, and the key's account in the credential
    /// store.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::Gemini => "gemini",
            Self::OpenAi => "openai",
        }
    }

    /// The model this service answers with.
    pub fn model(self, ai: &crate::config::Ai) -> &str {
        match self {
            Self::Anthropic => &ai.model,
            Self::Gemini => &ai.gemini_model,
            Self::OpenAi => &ai.openai_model,
        }
    }
}

/// A model a service offers, for the settings to list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AiModel {
    pub id: String,
    pub name: String,
}

/// Gemini's ways of saying it would not answer.
const GEMINI_BLOCKED: &[&str] = &[
    "SAFETY",
    "PROHIBITED_CONTENT",
    "BLOCKLIST",
    "SPII",
    "RECITATION",
    "IMAGE_SAFETY",
];

/// Whether a Gemini response, or one chunk of a stream, was blocked.
pub fn gemini_blocked(value: &serde_json::Value) -> bool {
    value.pointer("/promptFeedback/blockReason").is_some()
        || value
            .pointer("/candidates/0/finishReason")
            .and_then(|reason| reason.as_str())
            .is_some_and(|reason| GEMINI_BLOCKED.contains(&reason))
}

/// The answer's text in a Gemini `generateContent` response. Parts marked as
/// thoughts are the model's reasoning, not its answer.
pub fn gemini_text(value: &serde_json::Value) -> String {
    value
        .pointer("/candidates/0/content/parts")
        .and_then(|parts| parts.as_array())
        .into_iter()
        .flatten()
        .filter(|part| part.get("thought").and_then(|thought| thought.as_bool()) != Some(true))
        .filter_map(|part| part.get("text").and_then(|text| text.as_str()))
        .collect()
}

/// A Gemini `generateContent` reply's text, or why there is none.
pub fn gemini_reply(value: &serde_json::Value) -> Result<String, AiError> {
    if gemini_blocked(value) {
        return Err(AiError::Refused);
    }
    let text = gemini_text(value);
    if text.trim().is_empty() {
        return Err(AiError::Unavailable);
    }
    Ok(text)
}

/// An OpenAI Responses reply's text, or why there is none. The text is in
/// `output_text` parts of `message` items; a refusal is a part of its own.
pub fn openai_reply(value: &serde_json::Value) -> Result<String, AiError> {
    let parts: Vec<&serde_json::Value> = value
        .get("output")
        .and_then(|output| output.as_array())
        .into_iter()
        .flatten()
        .filter(|item| item.get("type").and_then(|kind| kind.as_str()) == Some("message"))
        .filter_map(|item| item.get("content").and_then(|content| content.as_array()))
        .flatten()
        .collect();

    let text: String = parts
        .iter()
        .filter(|part| part.get("type").and_then(|kind| kind.as_str()) == Some("output_text"))
        .filter_map(|part| part.get("text").and_then(|text| text.as_str()))
        .collect();

    if text.trim().is_empty() {
        let refused = parts
            .iter()
            .any(|part| part.get("type").and_then(|kind| kind.as_str()) == Some("refusal"));
        return Err(if refused {
            AiError::Refused
        } else {
            AiError::Unavailable
        });
    }
    Ok(text)
}

/// The models a service's model list offers for chatting, newest first where
/// the service says which is newest.
///
/// ponytail: what is a chat model is told from the id for Gemini and OpenAI,
/// whose lists also carry speech, image and embedding models; a new kind of
/// non-chat model shows up in the list until it gets a word here.
pub fn models_from(provider: Provider, value: &serde_json::Value) -> Vec<AiModel> {
    const NOT_CHAT: &[&str] = &[
        "audio",
        "realtime",
        "tts",
        "transcribe",
        "image",
        "embedding",
        "search",
        "instruct",
        "moderation",
        "aqa",
    ];
    let chat = |id: &str| !NOT_CHAT.iter().any(|word| id.contains(word));
    let text = |item: &serde_json::Value, key: &str| {
        item.get(key)
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_owned()
    };

    match provider {
        // Already newest first.
        Provider::Anthropic => value
            .get("data")
            .and_then(|data| data.as_array())
            .into_iter()
            .flatten()
            .map(|item| AiModel {
                id: text(item, "id"),
                name: text(item, "display_name"),
            })
            .filter(|model| !model.id.is_empty())
            .collect(),

        Provider::Gemini => value
            .get("models")
            .and_then(|models| models.as_array())
            .into_iter()
            .flatten()
            .filter(|item| {
                item.get("supportedGenerationMethods")
                    .and_then(|methods| methods.as_array())
                    .is_some_and(|methods| {
                        methods
                            .iter()
                            .any(|method| method.as_str() == Some("generateContent"))
                    })
            })
            .map(|item| AiModel {
                id: text(item, "name").trim_start_matches("models/").to_owned(),
                name: text(item, "displayName"),
            })
            .filter(|model| !model.id.is_empty() && chat(&model.id))
            .collect(),

        Provider::OpenAi => {
            let mut found: Vec<(i64, AiModel)> = value
                .get("data")
                .and_then(|data| data.as_array())
                .into_iter()
                .flatten()
                .filter_map(|item| {
                    let id = text(item, "id");
                    let family = id.starts_with("gpt-")
                        || id.starts_with("chatgpt-")
                        || (id.starts_with('o')
                            && id.chars().nth(1).is_some_and(|c| c.is_ascii_digit()));
                    (family && chat(&id)).then(|| {
                        let created = item.get("created").and_then(|c| c.as_i64()).unwrap_or(0);
                        (
                            created,
                            AiModel {
                                name: id.clone(),
                                id,
                            },
                        )
                    })
                })
                .collect();
            found.sort_by(|a, b| b.0.cmp(&a.0));
            found.into_iter().map(|(_, model)| model).collect()
        }
    }
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

    #[test]
    fn a_provider_round_trips_and_an_unknown_one_is_the_default() {
        for provider in [Provider::Anthropic, Provider::Gemini, Provider::OpenAi] {
            assert_eq!(Provider::parse(provider.as_str()), provider);
        }
        assert_eq!(Provider::parse("gemnii"), Provider::Anthropic);
    }

    #[test]
    fn a_gemini_wrong_key_is_a_bad_key_although_it_is_a_400() {
        let body = r#"{"error":{"code":400,"status":"INVALID_ARGUMENT",
            "details":[{"reason":"API_KEY_INVALID"}]}}"#;
        assert_eq!(AiError::from_response(400, body), AiError::BadKey);
        assert_eq!(AiError::from_response(400, "{}"), AiError::Unavailable);
    }

    #[test]
    fn a_gemini_reply_is_its_text_without_the_thoughts() {
        let value = serde_json::json!({"candidates":[{"content":{"parts":[
            {"text":"thinking it over","thought":true},{"text":"Bon"},{"text":"jour"}]},
            "finishReason":"STOP"}]});
        assert_eq!(gemini_reply(&value).unwrap(), "Bonjour");

        let blocked = serde_json::json!({"promptFeedback":{"blockReason":"SAFETY"}});
        assert_eq!(gemini_reply(&blocked), Err(AiError::Refused));
        let stopped = serde_json::json!({"candidates":[{"finishReason":"PROHIBITED_CONTENT"}]});
        assert_eq!(gemini_reply(&stopped), Err(AiError::Refused));
    }

    #[test]
    fn an_openai_reply_is_its_output_text_and_a_refusal_is_told_apart() {
        let value = serde_json::json!({"output":[
            {"type":"reasoning","summary":[]},
            {"type":"web_search_call","status":"completed"},
            {"type":"message","content":[{"type":"output_text","text":"Bonjour","annotations":[]}]}]});
        assert_eq!(openai_reply(&value).unwrap(), "Bonjour");

        let refused = serde_json::json!({"output":[
            {"type":"message","content":[{"type":"refusal","refusal":"No."}]}]});
        assert_eq!(openai_reply(&refused), Err(AiError::Refused));
        assert_eq!(
            openai_reply(&serde_json::json!({})),
            Err(AiError::Unavailable)
        );
    }

    #[test]
    fn the_model_lists_keep_the_chat_models_only() {
        let gemini = serde_json::json!({"models":[
            {"name":"models/gemini-3.8-flash","displayName":"Gemini 3.8 Flash",
             "supportedGenerationMethods":["generateContent","countTokens"]},
            {"name":"models/gemini-3.8-flash-tts","displayName":"TTS",
             "supportedGenerationMethods":["generateContent"]},
            {"name":"models/text-embedding-004","displayName":"Embedding",
             "supportedGenerationMethods":["embedContent"]}]});
        assert_eq!(
            models_from(Provider::Gemini, &gemini),
            [AiModel {
                id: "gemini-3.8-flash".into(),
                name: "Gemini 3.8 Flash".into()
            }]
        );

        let openai = serde_json::json!({"data":[
            {"id":"gpt-5.5","created":10},
            {"id":"whisper-1","created":30},
            {"id":"gpt-realtime","created":40},
            {"id":"o4-mini","created":5},
            {"id":"omni-moderation-latest","created":50},
            {"id":"gpt-6-sol","created":20}]});
        let ids: Vec<String> = models_from(Provider::OpenAi, &openai)
            .into_iter()
            .map(|model| model.id)
            .collect();
        assert_eq!(ids, ["gpt-6-sol", "gpt-5.5", "o4-mini"]);

        let anthropic = serde_json::json!({"data":[
            {"id":"claude-opus-5","display_name":"Claude Opus 5"}]});
        assert_eq!(
            models_from(Provider::Anthropic, &anthropic)[0].name,
            "Claude Opus 5"
        );
    }
}
