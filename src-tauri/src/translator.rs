//! Google Translate + DeepSeek paraphrasing module.

use std::time::Duration;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{deepseek, settings::Settings};

#[derive(Debug, Error)]
pub enum TranslateError {
    #[error("No text provided")]
    EmptyText,
    #[error("Network error: {0}")]
    Network(String),
    #[error("Translation API error ({status})")]
    ApiError { status: u16 },
    #[error("Failed to parse translation response")]
    ParseError,
    #[error("DeepSeek error: {0}")]
    DeepSeek(#[from] deepseek::DeepSeekError),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranslationResult {
    pub success: bool,
    pub translation: String,
    pub raw_translation: String,
    pub detected_language: String,
    pub target_language: String,
}

/// Call free Google Translate web API (gtx client).
pub async fn google_translate(
    text: &str,
    target_lang: &str,
) -> Result<(String, String), TranslateError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(TranslateError::EmptyText);
    }

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| TranslateError::Network(e.to_string()))?;

    let url = "https://translate.googleapis.com/translate_a/single";
    let resp = client
        .get(url)
        .query(&[
            ("client", "gtx"),
            ("sl", "auto"),
            ("tl", target_lang),
            ("dt", "t"),
            ("q", trimmed),
        ])
        .send()
        .await
        .map_err(|e| TranslateError::Network(e.to_string()))?;

    if !resp.status().is_success() {
        return Err(TranslateError::ApiError {
            status: resp.status().as_u16(),
        });
    }

    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|_| TranslateError::ParseError)?;

    // Parse translated text from result[0]
    let mut translation = String::new();
    if let Some(sentences) = json.get(0).and_then(|v| v.as_array()) {
        for s in sentences {
            if let Some(chunk) = s.get(0).and_then(|v| v.as_str()) {
                translation.push_str(chunk);
            }
        }
    }

    if translation.is_empty() {
        return Err(TranslateError::ParseError);
    }

    // Detected language from result[2]
    let detected_lang = json
        .get(2)
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    Ok((translation, detected_lang))
}

/// Translate text using Google Translate, then paraphrase with DeepSeek for clarity.
pub async fn translate_and_paraphrase<F>(
    api_key: &str,
    settings: &Settings,
    text: &str,
    target_lang: &str,
    mut on_delta: F,
) -> Result<TranslationResult, TranslateError>
where
    F: FnMut(&str),
{
    // Step 1: Translate using free Google Translate API
    let (raw_translation, detected_lang) = google_translate(text, target_lang).await?;

    // Step 2: Paraphrase using DeepSeek for clarity (if enabled and API key is available)
    let paraphrased = if settings.paraphrase_translation && !api_key.trim().is_empty() {
        let prompt = format!(
            "You are a professional translator and editor. Refine the following translated text for maximum clarity, natural phrasing, and 100% semantic fidelity in {target_lang}.\n\nStrict Rules:\n- Maintain absolute accuracy to the original meaning.\n- Do NOT alter proper nouns, technical terms, dates, numbers, or core facts.\n- Do NOT summarize or add extra commentary.\n- Return ONLY the final refined translation.\n\nText:\n{raw_translation}"
        );

        match deepseek::stream_rewrite(
            deepseek::RewriteRequest {
                api_key,
                settings,
                input: &prompt,
            },
            |delta| {
                on_delta(delta);
            },
        )
        .await
        {
            Ok(full) => full,
            Err(e) => {
                log::warn!("DeepSeek paraphrase failed ({e}); falling back to raw Google Translation");
                on_delta(&raw_translation);
                raw_translation.clone()
            }
        }
    } else {
        on_delta(&raw_translation);
        raw_translation.clone()
    };

    Ok(TranslationResult {
        success: true,
        translation: paraphrased,
        raw_translation,
        detected_language: detected_lang,
        target_language: target_lang.to_string(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeReplyResult {
    pub success: bool,
    pub reply: String,
    pub detected_language: String,
    pub detected_language_name: String,
}

/// Convert ISO language code into a human-readable language name.
pub fn lang_code_to_name(code: &str) -> &'static str {
    let clean = code.split('-').next().unwrap_or(code).to_lowercase();
    match clean.as_str() {
        "id" | "ind" => "Indonesian",
        "es" => "Spanish",
        "ja" => "Japanese",
        "zh" => "Chinese",
        "fr" => "French",
        "de" => "German",
        "ko" => "Korean",
        "pt" => "Portuguese",
        "ru" => "Russian",
        "ar" => "Arabic",
        "hi" => "Hindi",
        "th" => "Thai",
        "vi" => "Vietnamese",
        "nl" => "Dutch",
        "it" => "Italian",
        "tr" => "Turkish",
        "pl" => "Polish",
        "sv" => "Swedish",
        "en" => "English",
        _ => "the native language of the message",
    }
}

/// Detect the native language of the message/draft and generate a natural reply in that native language.
pub async fn generate_native_reply<F>(
    api_key: &str,
    settings: &Settings,
    text: &str,
    target_lang_override: Option<&str>,
    mut on_delta: F,
) -> Result<NativeReplyResult, TranslateError>
where
    F: FnMut(&str),
{
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(TranslateError::EmptyText);
    }

    // Step 1: Detect the language of the incoming message or text
    let (_, detected_lang_code) = google_translate(trimmed, "en")
        .await
        .unwrap_or_else(|_| (String::new(), "unknown".into()));

    let target_lang_code = target_lang_override.unwrap_or(&detected_lang_code);
    let lang_name = lang_code_to_name(target_lang_code);

    // Step 2: Use DeepSeek to craft an authentic, natural reply in the native language
    let prompt = format!(
        "You are an expert native speaker and writing assistant in {lang_name}.\n\
         Generate or translate a natural, friendly, and contextually fluent reply in {lang_name} for the following message/draft.\n\n\
         Rules:\n\
         - Match the tone, slang, and cultural nuances of conversational {lang_name}.\n\
         - Keep it authentic, engaging, and natural (e.g. for Indonesian, use natural conversational tone like 'Siap, ayo gas bro!').\n\
         - Do not include explanations, meta commentary, or quotes.\n\
         - Output ONLY the final native reply.\n\n\
         Message/Draft:\n{trimmed}"
    );

    let reply = if !api_key.trim().is_empty() {
        match deepseek::stream_rewrite(
            deepseek::RewriteRequest {
                api_key,
                settings,
                input: &prompt,
            },
            |delta| {
                on_delta(delta);
            },
        )
        .await
        {
            Ok(full) => full,
            Err(e) => {
                log::warn!("DeepSeek native reply generation failed ({e}); falling back to Google Translate");
                let (translated, _) = google_translate(trimmed, target_lang_code).await?;
                on_delta(&translated);
                translated
            }
        }
    } else {
        let (translated, _) = google_translate(trimmed, target_lang_code).await?;
        on_delta(&translated);
        translated
    };

    Ok(NativeReplyResult {
        success: true,
        reply,
        detected_language: detected_lang_code,
        detected_language_name: lang_name.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_translation_result_camel_case() {
        let res = TranslationResult {
            success: true,
            translation: "Hello world".into(),
            raw_translation: "Hola mundo".into(),
            detected_language: "es".into(),
            target_language: "en".into(),
        };
        let json = serde_json::to_string(&res).unwrap();
        assert!(json.contains("\"success\":true"));
        assert!(json.contains("\"rawTranslation\":\"Hola mundo\""));
        assert!(json.contains("\"detectedLanguage\":\"es\""));
        assert!(json.contains("\"targetLanguage\":\"en\""));
    }

    #[test]
    fn maps_language_codes_correctly() {
        assert_eq!(lang_code_to_name("id"), "Indonesian");
        assert_eq!(lang_code_to_name("es"), "Spanish");
        assert_eq!(lang_code_to_name("ja"), "Japanese");
        assert_eq!(lang_code_to_name("zh-CN"), "Chinese");
    }

    #[tokio::test]
    async fn google_translate_rejects_empty_input() {
        let res = google_translate("   ", "en").await;
        assert!(matches!(res, Err(TranslateError::EmptyText)));
    }
}
