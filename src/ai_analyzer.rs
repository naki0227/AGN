//! AGN AI Semantic Analyzer - AIセマンティック・アナライザ
//! Gemini APIを用いた意図推論（オプショナル）

use serde::{Deserialize, Serialize};

/// AI分析リクエスト
#[derive(Debug, Serialize)]
pub struct AnalysisRequest {
    pub code: String,
    pub error_message: String,
}

/// AI分析レスポンス
#[derive(Debug, Deserialize)]
pub struct AnalysisResponse {
    pub corrected_code: String,
    pub explanation: String,
    pub confidence: f64,
}

/// AIアナライザの設定
pub struct AiAnalyzerConfig {
    pub api_key: Option<String>,
    pub model: String,
    pub enabled: bool,
}

impl Default for AiAnalyzerConfig {
    fn default() -> Self {
        Self {
            api_key: std::env::var("GEMINI_API_KEY").ok(),
            model: "gemini-2.0-flash".to_string(),
            enabled: false, // デフォルトは無効
        }
    }
}

pub struct AiAnalyzer {
    config: AiAnalyzerConfig,
}

impl AiAnalyzer {
    pub fn new(config: AiAnalyzerConfig) -> Self {
        Self { config }
    }

    /// AI分析が有効かどうか
    pub fn is_enabled(&self) -> bool {
        self.config.enabled && self.config.api_key.is_some()
    }

    /// パースエラー時にAIで意図を推論（将来の拡張用）
    #[allow(dead_code)]
    pub async fn analyze_error(&self, code: &str, error: &str) -> Option<AnalysisResponse> {
        if !self.is_enabled() {
            return None;
        }

        let _api_key = self.config.api_key.as_ref()?;
        
        // Gemini API呼び出し（将来実装）
        // 現在はプレースホルダー
        let prompt = format!(
            r#"以下のAGN言語コードにエラーがあります。ユーザーの意図を推測し、正しいコードを出力してください。

エラー: {}

コード:
{}

出力形式:
1. 修正後のコード
2. 修正理由"#,
            error, code
        );

        println!("[AI Analyzer] Would send prompt: {}", prompt);
        
        // モック応答
        Some(AnalysisResponse {
            corrected_code: code.to_string(),
            explanation: "AI analysis not yet implemented".to_string(),
            confidence: 0.0,
        })
    }
}

impl Default for AiAnalyzer {
    fn default() -> Self {
        Self::new(AiAnalyzerConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disabled_by_default() {
        let analyzer = AiAnalyzer::default();
        assert!(!analyzer.is_enabled());
    }

    #[test]
    fn test_enabled_with_api_key() {
        let config = AiAnalyzerConfig {
            api_key: Some("test-key".to_string()),
            model: "gemini-2.0-flash".to_string(),
            enabled: true,
        };
        let analyzer = AiAnalyzer::new(config);
        assert!(analyzer.is_enabled());
    }
}
