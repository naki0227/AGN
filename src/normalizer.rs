//! AGN Normalizer - コード正規化器
//! 曖昧な入力を正規化し、補正ログを出力する

use std::fmt;

/// 補正の種類
#[derive(Debug, Clone)]
pub enum CorrectionType {
    /// 句読点の除去
    PunctuationRemoved(char),
    /// 動詞エイリアスの解決
    VerbAlias { from: String, to: String },
    /// 助詞の補正
    ParticleCorrection { from: String, to: String },
    /// 全角スペースの変換
    FullWidthSpace,
}

impl fmt::Display for CorrectionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CorrectionType::PunctuationRemoved(c) => write!(f, "Removed punctuation: {}", c),
            CorrectionType::VerbAlias { from, to } => write!(f, "Verb alias: {} → {}", from, to),
            CorrectionType::ParticleCorrection { from, to } => write!(f, "Particle: {} → {}", from, to),
            CorrectionType::FullWidthSpace => write!(f, "Full-width space → half-width"),
        }
    }
}

/// 1行の補正記録
#[derive(Debug, Clone)]
pub struct LineCorrection {
    pub line_number: usize,
    pub original: String,
    pub normalized: String,
    pub corrections: Vec<CorrectionType>,
}

impl fmt::Display for LineCorrection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.corrections.is_empty() {
            return Ok(());
        }
        write!(f, "[NORMALIZE] Line {}: \"{}\" → \"{}\"\n", 
            self.line_number, self.original.trim(), self.normalized.trim())?;
        for correction in &self.corrections {
            write!(f, "  - {}\n", correction)?;
        }
        Ok(())
    }
}

/// 動詞エイリアスマッピング
/// 注意: 順序が重要。部分一致を避けるため、長い文字列を先に配置
const VERB_ALIASES: &[(&str, &str)] = &[
    // ひらがな → 漢字
    ("ひょうじする", "表示する"),
    ("たす", "足す"),
    ("ひく", "引く"),
    ("かける", "掛ける"),
    ("わる", "割る"),
    // 類義語 → 標準形（「表示する」を含まないものだけ）
    ("出す", "表示する"),
    ("見せる", "表示する"),
    ("プリントする", "表示する"),
    ("印刷する", "表示する"),
    ("加える", "足す"),
    ("加算する", "足す"),
    ("減算する", "引く"),
    ("乗算する", "掛ける"),
    ("除算する", "割る"),
];

/// 助詞補正マッピング
const PARTICLE_CORRECTIONS: &[(&str, &str)] = &[
    ("が", "は"),  // 主語マーカーの統一（文脈による）
];

/// 除去する句読点
const PUNCTUATION: &[char] = &['。', '、', '！', '？', '．', '，'];

pub struct Normalizer {
    verbose: bool,
}

impl Normalizer {
    pub fn new() -> Self {
        Self { verbose: true }
    }

    #[allow(dead_code)]
    pub fn set_verbose(&mut self, verbose: bool) {
        self.verbose = verbose;
    }

    /// コード全体を正規化
    pub fn normalize(&self, code: &str) -> (String, Vec<LineCorrection>) {
        let mut normalized_lines = Vec::new();
        let mut all_corrections = Vec::new();

        for (i, line) in code.lines().enumerate() {
            let (normalized, correction) = self.normalize_line(i + 1, line);
            normalized_lines.push(normalized);
            if !correction.corrections.is_empty() {
                all_corrections.push(correction);
            }
        }

        (normalized_lines.join("\n"), all_corrections)
    }

    /// 1行を正規化
    fn normalize_line(&self, line_number: usize, line: &str) -> (String, LineCorrection) {
        let original = line.to_string();
        let mut result = line.to_string();
        let mut corrections = Vec::new();

        // 1. 全角スペース → 半角スペース
        if result.contains('　') {
            result = result.replace('　', " ");
            corrections.push(CorrectionType::FullWidthSpace);
        }

        // 2. 句読点の除去
        for &punct in PUNCTUATION {
            if result.contains(punct) {
                result = result.replace(punct, "");
                corrections.push(CorrectionType::PunctuationRemoved(punct));
            }
        }

        // 3. 動詞エイリアスの解決
        for (from, to) in VERB_ALIASES {
            if result.contains(from) {
                result = result.replace(from, to);
                corrections.push(CorrectionType::VerbAlias {
                    from: from.to_string(),
                    to: to.to_string(),
                });
            }
        }

        // 4. 助詞の補正（文脈依存のため慎重に）
        // 「X が 10 だ」→「X は 10 だ」のパターンのみ補正
        for (from, to) in PARTICLE_CORRECTIONS {
            let pattern = format!(" {} ", from);
            let replacement = format!(" {} ", to);
            if result.contains(&pattern) {
                // 「だ」で終わる代入文のみ補正
                if result.trim().ends_with("だ") {
                    result = result.replace(&pattern, &replacement);
                    corrections.push(CorrectionType::ParticleCorrection {
                        from: from.to_string(),
                        to: to.to_string(),
                    });
                }
            }
        }

        let normalized = result.trim().to_string();
        
        (
            normalized.clone(),
            LineCorrection {
                line_number,
                original,
                normalized,
                corrections,
            },
        )
    }

    /// 補正ログを整形して出力
    pub fn format_corrections(&self, corrections: &[LineCorrection]) -> String {
        if corrections.is_empty() {
            return String::new();
        }

        let mut output = String::from("=== Normalization Log ===\n");
        for correction in corrections {
            output.push_str(&format!("{}", correction));
        }
        output
    }
}

impl Default for Normalizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_punctuation_removal() {
        let normalizer = Normalizer::new();
        let (normalized, corrections) = normalizer.normalize("X は 10 だ。");
        assert_eq!(normalized, "X は 10 だ");
        assert_eq!(corrections.len(), 1);
    }

    #[test]
    fn test_verb_alias() {
        let normalizer = Normalizer::new();
        let (normalized, corrections) = normalizer.normalize("X に 5 を たす");
        assert_eq!(normalized, "X に 5 を 足す");
        assert_eq!(corrections.len(), 1);
    }

    #[test]
    fn test_verb_alias_display() {
        let normalizer = Normalizer::new();
        let (normalized, corrections) = normalizer.normalize("X を 出す");
        assert_eq!(normalized, "X を 表示する");
        assert_eq!(corrections.len(), 1);
    }

    #[test]
    fn test_multiple_corrections() {
        let normalizer = Normalizer::new();
        let code = "X は 10 だ。\nX に 5 を たす。\nX を 並列で 出す。";
        let (normalized, corrections) = normalizer.normalize(code);
        
        let expected = "X は 10 だ\nX に 5 を 足す\nX を 並列で 表示する";
        assert_eq!(normalized, expected);
        assert_eq!(corrections.len(), 3);
    }

    #[test]
    fn test_particle_correction() {
        let normalizer = Normalizer::new();
        let (normalized, _) = normalizer.normalize("X が 10 だ");
        assert_eq!(normalized, "X は 10 だ");
    }
}
