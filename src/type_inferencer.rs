//! AGN Type Inferencer - 意図ベースの型推論器
//! コード全体をスキャンして変数の型と生存期間を予測する
//! Eeyo: 次元解析（距離・時間の型安全性）

use crate::parser::{Expr, Program, Statement};
use serde::{Deserialize, Serialize};

/// 推論された型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InferredType {
    Number,
    String,
    Unknown,
    // Eeyo: 空間・時間型 (Phase 13)
    Distance { unit: String },  // "m", "km"
    Duration { unit: String },  // "秒", "分", "時間"
}

impl std::fmt::Display for InferredType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InferredType::Number => write!(f, "Number"),
            InferredType::String => write!(f, "String"),
            InferredType::Unknown => write!(f, "Unknown"),
            InferredType::Distance { unit } => write!(f, "Distance({})", unit),
            InferredType::Duration { unit } => write!(f, "Duration({})", unit),
        }
    }
}

/// 変数の生存期間
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lifetime {
    /// 最初に登場する行
    pub start: usize,
    /// 最後に参照される行
    pub end: usize,
}

/// 変数のメタデータ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableMetadata {
    /// 変数名
    pub name: String,
    /// 推論された型
    pub inferred_type: InferredType,
    /// 生存期間
    pub lifetime: Lifetime,
    /// 推論の確信度 (0.0 - 1.0)
    pub confidence: f64,
    /// 型が決定された理由
    pub reason: String,
}

/// 型推論の結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeInferenceResult {
    pub variables: Vec<VariableMetadata>,
}

impl TypeInferenceResult {
    /// JSON形式で出力
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// 人間が読みやすい形式で出力
    pub fn to_human_readable(&self) -> String {
        let mut output = String::from("=== Type Inference ===\n");
        for var in &self.variables {
            output.push_str(&format!(
                "Variable \"{}\": {} (confidence: {:.2}), lifetime: lines {}-{}\n",
                var.name, var.inferred_type, var.confidence, var.lifetime.start, var.lifetime.end
            ));
            output.push_str(&format!("  Reason: {}\n", var.reason));
        }
        output
    }
}

pub struct TypeInferencer;

impl TypeInferencer {
    pub fn new() -> Self {
        Self
    }

    /// プログラム全体から型を推論
    pub fn infer(&self, program: &Program) -> TypeInferenceResult {
        let mut variables: std::collections::HashMap<String, VariableMetadata> = 
            std::collections::HashMap::new();

        for (line_idx, stmt) in program.statements.iter().enumerate() {
            let line_num = line_idx + 1;
            self.process_statement(stmt, line_num, &mut variables);
        }

        TypeInferenceResult {
            variables: variables.into_values().collect(),
        }
    }

    fn process_statement(
        &self,
        stmt: &Statement,
        line_num: usize,
        variables: &mut std::collections::HashMap<String, VariableMetadata>,
    ) {
        match stmt {
            Statement::Assignment { name, value } => {
                let (inferred_type, confidence, reason) = self.infer_from_expr(value);
                
                variables.insert(name.clone(), VariableMetadata {
                    name: name.clone(),
                    inferred_type,
                    lifetime: Lifetime { start: line_num, end: line_num },
                    confidence,
                    reason,
                });
            }
            Statement::LoadAsset { target, path: _ } => {
                variables.insert(target.clone(), VariableMetadata {
                    name: target.clone(),
                    inferred_type: InferredType::String, // Assets path treated as string/image
                    lifetime: Lifetime { start: line_num, end: line_num },
                    confidence: 0.9,
                    reason: "Asset loaded".to_string(),
                });
            }
            Statement::ComponentDefine { target, style: _, component: _ } => {
                variables.insert(target.clone(), VariableMetadata {
                    name: target.clone(),
                    inferred_type: InferredType::Unknown, // Component type
                    lifetime: Lifetime { start: line_num, end: line_num },
                    confidence: 0.8,
                    reason: "UI Component defined".to_string(),
                });
            }
            Statement::BinaryOp { target, operand, verb } => {
                // 演算対象は数値型であるべき
                if let Some(var) = variables.get_mut(target) {
                    var.lifetime.end = line_num;
                    
                    // 演算動詞から型を強化推論
                    if matches!(verb.as_str(), "足す" | "引く" | "掛ける" | "割る") {
                        if var.inferred_type == InferredType::Unknown {
                            var.inferred_type = InferredType::Number;
                            var.confidence = 0.9;
                            var.reason = format!("Used in arithmetic operation: {}", verb);
                        }
                    }
                }
                
                // オペランドが変数なら生存期間を更新
                if let Expr::Variable(op_name) = operand {
                    if let Some(var) = variables.get_mut(op_name) {
                        var.lifetime.end = line_num;
                    }
                }
            }
            Statement::UnaryOp { operand, verb: _ } | Statement::AsyncOp { operand, verb: _ } => {
                if let Expr::Variable(name) = operand {
                    if let Some(var) = variables.get_mut(name) {
                        var.lifetime.end = line_num;
                    }
                }
            }
            Statement::IfStatement { condition: _, then_block, else_block } => {
                // Process statements in then block
                for (idx, inner_stmt) in then_block.iter().enumerate() {
                    self.process_statement(inner_stmt, line_num + idx, variables);
                }
                // Process statements in else block
                if let Some(else_stmts) = else_block {
                    for (i, inner_stmt) in else_stmts.iter().enumerate() {
                        self.process_statement(inner_stmt, line_num + i, variables);
                    }
                }
            }
            Statement::RepeatStatement { count: _, body } => {
                // Process statements in loop body
                for (idx, inner_stmt) in body.iter().enumerate() {
                    self.process_statement(inner_stmt, line_num + idx, variables);
                }
            }
            Statement::AiOp { result, input: _, verb: _, options: _ } => {
                // AI操作の結果は常にString
                variables.insert(result.clone(), VariableMetadata {
                    name: result.clone(),
                    inferred_type: InferredType::String,
                    lifetime: Lifetime { start: line_num, end: line_num },
                    confidence: 1.0,
                    reason: "Result of AI operation".to_string(),
                });
            }
            Statement::ScreenOp { operand: _ } => {
                // Screen出力は変数を更新しない
            }
            Statement::EventHandler { target: _, event: _, body } => {
                // Process statements in event handler body
                for (idx, inner_stmt) in body.iter().enumerate() {
                    self.process_statement(inner_stmt, line_num + idx, variables);
                }
            }
            Statement::Block { target, body } => {
                // Block operates on target (ensure implicit usage)
                if let Some(var) = variables.get_mut(target) {
                    var.lifetime.end = line_num + body.len();
                }
                
                // Process body
                for (idx, inner_stmt) in body.iter().enumerate() {
                    self.process_statement(inner_stmt, line_num + idx, variables);
                }
            }
            Statement::Layout { target, direction: _ } => {
                // Layout updates target
                if let Some(var) = variables.get_mut(target) {
                    var.lifetime.end = line_num;
                    var.reason = "Layout applied".to_string();
                }
            }
            Statement::DelayStatement { body, .. } => {
                // Analyze body
                for s in body {
                    self.process_statement(s, line_num, variables);
                }
            }
            Statement::AnimateStatement { value, .. } => {
                // Animation affects UI prop and might use variable
                if let Expr::Variable(name) = value {
                    if let Some(var) = variables.get_mut(name) {
                        var.lifetime.end = line_num;
                    }
                }
            }
            // Eeyo: 空間ステートメント（後方互換性のためスキップ）
            Statement::SpatialSearch { result, .. } => {
                variables.insert(result.clone(), VariableMetadata {
                    name: result.clone(),
                    inferred_type: InferredType::Unknown,
                    lifetime: Lifetime { start: line_num, end: line_num },
                    confidence: 0.5,
                    reason: "空間検索結果".to_string(),
                });
            }
            Statement::BeaconBroadcast { .. } | Statement::Notify { .. } | Statement::TokuAccrue { .. } => {
                // Eeyo用の空間ステートメント（型推論不要）
            }
        }
    }

    fn infer_from_expr(&self, expr: &Expr) -> (InferredType, f64, String) {
        match expr {
            Expr::Number(_) => (
                InferredType::Number,
                1.0,
                "Assigned from number literal".to_string(),
            ),
            Expr::String(_) => (
                InferredType::String,
                1.0,
                "Assigned from string literal".to_string(),
            ),
            Expr::Variable(name) => (
                InferredType::Unknown,
                0.5,
                format!("Assigned from variable '{}' (type unknown)", name),
            ),
            // Eeyo: 空間・時間型（次元解析対応）
            Expr::Distance { value: _, unit } => (
                InferredType::Distance { unit: unit.clone() },
                1.0,
                format!("Distance literal with unit '{}'", unit),
            ),
            Expr::Duration { value: _, unit } => (
                InferredType::Duration { unit: unit.clone() },
                1.0,
                format!("Duration literal with unit '{}'", unit),
            ),
        }
    }

    /// 次元解析: 2つの型が演算可能かチェック
    pub fn check_dimension_compatibility(left: &InferredType, right: &InferredType) -> Result<InferredType, String> {
        match (left, right) {
            // 同じ型同士は演算可能
            (InferredType::Number, InferredType::Number) => Ok(InferredType::Number),
            (InferredType::String, InferredType::String) => Ok(InferredType::String),
            
            // 距離同士は演算可能（単位変換が必要な場合あり）
            (InferredType::Distance { unit: u1 }, InferredType::Distance { unit: u2 }) => {
                if u1 == u2 {
                    Ok(InferredType::Distance { unit: u1.clone() })
                } else {
                    Err(format!("次元エラー: 距離の単位が不一致 ({} vs {})", u1, u2))
                }
            }
            
            // 時間同士は演算可能
            (InferredType::Duration { unit: u1 }, InferredType::Duration { unit: u2 }) => {
                if u1 == u2 {
                    Ok(InferredType::Duration { unit: u1.clone() })
                } else {
                    Err(format!("次元エラー: 時間の単位が不一致 ({} vs {})", u1, u2))
                }
            }
            
            // 距離と数値の乗除は可能（スケーリング）
            (InferredType::Distance { unit }, InferredType::Number) |
            (InferredType::Number, InferredType::Distance { unit }) => {
                Ok(InferredType::Distance { unit: unit.clone() })
            }
            
            // 時間と数値の乗除は可能（スケーリング）
            (InferredType::Duration { unit }, InferredType::Number) |
            (InferredType::Number, InferredType::Duration { unit }) => {
                Ok(InferredType::Duration { unit: unit.clone() })
            }
            
            // 距離と時間の混合は禁止（物理的に無意味）
            (InferredType::Distance { .. }, InferredType::Duration { .. }) |
            (InferredType::Duration { .. }, InferredType::Distance { .. }) => {
                Err("次元エラー: 距離と時間は直接演算できません".to_string())
            }
            
            // その他は不明
            _ => Ok(InferredType::Unknown),
        }
    }
}

impl Default for TypeInferencer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    #[test]
    fn test_infer_number() {
        let code = "X は 10 だ";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        let inferencer = TypeInferencer::new();
        let result = inferencer.infer(&program);
        
        assert_eq!(result.variables.len(), 1);
        assert_eq!(result.variables[0].inferred_type, InferredType::Number);
        assert_eq!(result.variables[0].confidence, 1.0);
    }

    #[test]
    fn test_infer_string() {
        let code = r#"メッセージ は "Hello" だ"#;
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        let inferencer = TypeInferencer::new();
        let result = inferencer.infer(&program);
        
        assert_eq!(result.variables.len(), 1);
        assert_eq!(result.variables[0].inferred_type, InferredType::String);
    }

    #[test]
    fn test_lifetime_tracking() {
        let code = "X は 10 だ\nX に 5 を 足す\nX を 表示する";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        let inferencer = TypeInferencer::new();
        let result = inferencer.infer(&program);
        
        assert_eq!(result.variables.len(), 1);
        assert_eq!(result.variables[0].lifetime.start, 1);
        assert_eq!(result.variables[0].lifetime.end, 3);
    }

    // === Eeyo: 次元解析テスト (Phase 13) ===

    #[test]
    fn test_dimension_distance_plus_distance() {
        let left = InferredType::Distance { unit: "m".to_string() };
        let right = InferredType::Distance { unit: "m".to_string() };
        let result = TypeInferencer::check_dimension_compatibility(&left, &right);
        assert!(result.is_ok());
    }

    #[test]
    fn test_dimension_distance_plus_duration_error() {
        let left = InferredType::Distance { unit: "m".to_string() };
        let right = InferredType::Duration { unit: "分".to_string() };
        let result = TypeInferencer::check_dimension_compatibility(&left, &right);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("次元エラー"));
    }

    #[test]
    fn test_dimension_distance_unit_mismatch() {
        let left = InferredType::Distance { unit: "m".to_string() };
        let right = InferredType::Distance { unit: "km".to_string() };
        let result = TypeInferencer::check_dimension_compatibility(&left, &right);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("単位が不一致"));
    }

    #[test]
    fn test_dimension_distance_times_number() {
        let left = InferredType::Distance { unit: "m".to_string() };
        let right = InferredType::Number;
        let result = TypeInferencer::check_dimension_compatibility(&left, &right);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), InferredType::Distance { unit: "m".to_string() });
    }
}
