//! AGN AI Runtime - AI動詞の実行ランタイム
//! Gemini API等を使用してAI処理を実行する

use std::env;

/// AIランタイムエラー
#[derive(Debug)]
pub enum AiError {
    ApiKeyNotSet,
    RequestFailed(String),
    ParseError(String),
}

impl std::fmt::Display for AiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AiError::ApiKeyNotSet => write!(f, "GEMINI_API_KEY environment variable not set"),
            AiError::RequestFailed(e) => write!(f, "API request failed: {}", e),
            AiError::ParseError(e) => write!(f, "Failed to parse response: {}", e),
        }
    }
}

/// AIランタイム設定
pub struct AiRuntime {
    api_key: Option<String>,
    model: String,
    enabled: bool,
}

impl AiRuntime {
    pub fn new() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let api_key = env::var("GEMINI_API_KEY").ok();
        #[cfg(target_arch = "wasm32")]
        let api_key = None;

        #[cfg(not(target_arch = "wasm32"))]
        let enabled = api_key.is_some();
        #[cfg(target_arch = "wasm32")]
        let enabled = true;

        Self {
            enabled,
            api_key,
            model: "gemini-2.0-flash".to_string(),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// 要約を実行
    pub async fn summarize(&self, text: &str) -> Result<String, AiError> {
        if !self.enabled {
            // APIキーがない場合はプレースホルダーを返す
            return Ok(format!("[要約: {}...]", &text.chars().take(20).collect::<String>()));
        }

        let prompt = format!(
            "以下のテキストを簡潔に要約してください。要約のみを回答し、他の説明は不要です。\n\n{}",
            text
        );
        
        self.call_gemini(&prompt).await
    }

    /// 翻訳を実行
    pub async fn translate(&self, text: &str, target_lang: &str) -> Result<String, AiError> {
        if !self.enabled {
            return Ok(format!("[翻訳({}): {}...]", target_lang, &text.chars().take(20).collect::<String>()));
        }

        let prompt = format!(
            "以下のテキストを{}に翻訳してください。翻訳結果のみを回答し、他の説明は不要です。\n\n{}",
            target_lang, text
        );
        
        self.call_gemini(&prompt).await
    }

    /// Gemini APIを呼び出し
    /// Gemini APIを呼び出し (Native)
    #[cfg(not(target_arch = "wasm32"))]
    async fn call_gemini(&self, prompt: &str) -> Result<String, AiError> {
        let api_key = self.api_key.as_ref().ok_or(AiError::ApiKeyNotSet)?;
        
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, api_key
        );

        let body = serde_json::json!({
            "contents": [{
                "parts": [{
                    "text": prompt
                }]
            }]
        });

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AiError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AiError::RequestFailed(error_text));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AiError::ParseError(e.to_string()))?;

        // レスポンスからテキストを抽出
        let text = json["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .ok_or_else(|| AiError::ParseError("No text in response".to_string()))?;

        Ok(text.trim().to_string())
    }

    /// Gemini APIを呼び出し (Wasm: Proxy to /api/gemini)
    #[cfg(target_arch = "wasm32")]
    async fn call_gemini(&self, prompt: &str) -> Result<String, AiError> {
        // Get origin from window.location for absolute URL
        let origin = web_sys::window()
            .and_then(|w| w.location().origin().ok())
            .unwrap_or_else(|| "http://localhost:3003".to_string());
        let url = format!("{}/api/gemini", origin);
        
        let body = serde_json::json!({
            "prompt": prompt,
            "model": self.model
        });

        let client = reqwest::Client::new();
        let response = client
            .post(url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AiError::RequestFailed(e.to_string()))?;

        if !response.status().is_success() {
             // For text() on wasm, we might need await? yes.
             // But on some error response it might be text.
            let error_text = response.text().await.unwrap_or_default();
            return Err(AiError::RequestFailed(error_text));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| AiError::ParseError(e.to_string()))?;

        // Proxy expected response: { "text": "..." }
        let text = json["text"]
            .as_str()
            .ok_or_else(|| AiError::ParseError("No text in response".to_string()))?;

        Ok(text.trim().to_string())
    }

    /// AI動詞を実行
    pub async fn execute_verb(&self, verb: &str, input: &str, option: Option<String>) -> Result<String, AiError> {
        match verb {
            "要約する" | "summarize" => self.summarize(input).await,
            "翻訳する" | "translate" => {
                 let target = option.unwrap_or_else(|| "英語".to_string());
                 self.translate(input, &target).await
            },
            _ => Err(AiError::RequestFailed(format!("Unknown AI verb: {}", verb))),
        }
    }
}

impl Default for AiRuntime {
    fn default() -> Self {
        Self::new()
    }
}

/// LLVM IR用のFFI宣言を生成
pub fn emit_ai_ffi_declarations() -> String {
    let mut ir = String::new();
    
    ir.push_str("; AI Runtime FFI declarations\n");
    ir.push_str("declare i8* @agn_ai_summarize(i8*)\n");
    ir.push_str("declare i8* @agn_ai_translate(i8*, i8*)\n");
    ir.push_str("declare void @agn_ai_free(i8*)\n");
    
    ir
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_creation() {
        let runtime = AiRuntime::new();
        // APIキーがなくても作成できる
        assert!(runtime.model == "gemini-2.0-flash");
    }

    #[tokio::test]
    async fn test_summarize_without_api_key() {
        let runtime = AiRuntime {
            api_key: None,
            model: "gemini-2.0-flash".to_string(),
            enabled: false,
        };
        
        let result = runtime.summarize("これはテストテキストです").await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("[要約:"));
    }
}
