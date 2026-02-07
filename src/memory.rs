//! AGN Memory Manager - メモリ管理
//! 変数の生存期間に基づくメモリ管理（ARC プロトタイプ）

use crate::type_inferencer::{InferredType, Lifetime, TypeInferenceResult};
use std::collections::HashMap;

/// メモリアロケーション情報
#[derive(Debug, Clone)]
pub struct Allocation {
    pub var_name: String,
    pub var_type: InferredType,
    pub lifetime: Lifetime,
    pub is_heap: bool,
}

/// メモリマネージャ
/// Phase 3ではスタック変数のみをサポート
pub struct MemoryManager {
    allocations: HashMap<String, Allocation>,
}

impl MemoryManager {
    pub fn new() -> Self {
        Self {
            allocations: HashMap::new(),
        }
    }

    /// 型推論結果からアロケーション情報を構築
    pub fn analyze(&mut self, type_info: &TypeInferenceResult) {
        for var in &type_info.variables {
            let is_heap = matches!(var.inferred_type, InferredType::String);
            
            self.allocations.insert(var.name.clone(), Allocation {
                var_name: var.name.clone(),
                var_type: var.inferred_type.clone(),
                lifetime: var.lifetime.clone(),
                is_heap,
            });
        }
    }

    /// 指定した行で終了する変数のクリーンアップ命令を生成
    #[allow(dead_code)]
    pub fn emit_cleanup_for_line(&self, line: usize) -> Vec<String> {
        let mut cleanup = Vec::new();
        
        for alloc in self.allocations.values() {
            if alloc.lifetime.end == line && alloc.is_heap {
                // ヒープ変数の解放（Phase 4で実装）
                cleanup.push(format!(
                    "    ; TODO: Free heap allocation for '{}'\n",
                    alloc.var_name
                ));
            }
        }
        
        cleanup
    }

    /// 変数がスコープを抜ける際の解放命令を生成
    #[allow(dead_code)]
    pub fn emit_cleanup(&self, var_name: &str) -> Option<String> {
        let alloc = self.allocations.get(var_name)?;
        
        if alloc.is_heap {
            // Phase 4で実装: 参照カウントのデクリメントと条件付き解放
            Some(format!(
                "    ; ARC: decrement refcount for '{}' and free if zero\n",
                var_name
            ))
        } else {
            // スタック変数は自動解放（命令不要）
            None
        }
    }

    /// すべての変数の最終クリーンアップ
    pub fn emit_final_cleanup(&self) -> String {
        let mut ir = String::new();
        
        for alloc in self.allocations.values() {
            if alloc.is_heap {
                ir.push_str(&format!(
                    "    ; Cleanup: {} ({})\n",
                    alloc.var_name, alloc.var_type
                ));
            }
        }
        
        ir
    }

    /// アロケーション統計を取得
    pub fn get_stats(&self) -> MemoryStats {
        let mut stats = MemoryStats::default();
        
        for alloc in self.allocations.values() {
            match &alloc.var_type {
                InferredType::Number => stats.stack_allocations += 1,
                InferredType::String => stats.heap_allocations += 1,
                InferredType::Unknown => stats.unknown += 1,
                // Eeyo: 空間・時間型はスタック割当
                InferredType::Distance { .. } => stats.stack_allocations += 1,
                InferredType::Duration { .. } => stats.stack_allocations += 1,
            }
        }
        
        stats
    }
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self::new()
    }
}

/// メモリ統計
#[derive(Debug, Default)]
pub struct MemoryStats {
    pub stack_allocations: usize,
    pub heap_allocations: usize,
    pub unknown: usize,
}

impl std::fmt::Display for MemoryStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Memory: {} stack, {} heap, {} unknown",
            self.stack_allocations, self.heap_allocations, self.unknown
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::type_inferencer::TypeInferencer;

    #[test]
    fn test_stack_allocation() {
        let code = "X は 10 だ\nY は 20 だ";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        let inferencer = TypeInferencer::new();
        let type_info = inferencer.infer(&program);
        
        let mut mm = MemoryManager::new();
        mm.analyze(&type_info);
        
        let stats = mm.get_stats();
        assert_eq!(stats.stack_allocations, 2);
        assert_eq!(stats.heap_allocations, 0);
    }

    #[test]
    fn test_heap_allocation() {
        let code = r#"メッセージ は "Hello" だ"#;
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        let inferencer = TypeInferencer::new();
        let type_info = inferencer.infer(&program);
        
        let mut mm = MemoryManager::new();
        mm.analyze(&type_info);
        
        let stats = mm.get_stats();
        assert_eq!(stats.heap_allocations, 1);
    }
}
