//! AGN Symbol Table - シンボルテーブル
//! O(1)でシンボルの登録・参照を行うハッシュマップ実装

use std::collections::HashMap;

/// 値の型
#[derive(Debug, Clone)]
pub enum Value {
    Number(f64),
    String(String),
    /// 画像アセット（パス）
    Image(String),
    /// UIコンポーネント
    Component {
        style: String,
        ty: String, // type is reserved
        label: Option<String>,
        children: Vec<Value>,
        layout: Option<String>, // "vertical" or "horizontal"
    },
    Nil,
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Number(n) => {
                // 整数として表示できる場合は整数で表示
                if n.fract() == 0.0 {
                    write!(f, "{}", *n as i64)
                } else {
                    write!(f, "{}", n)
                }
            }
            Value::String(s) => write!(f, "{}", s),
            Value::Image(path) => write!(f, "[Image: {}]", path),
            Value::Component { style, ty, label, children, .. } => {
                let count = children.len();
                // Check for string children to override content
                let mut content = label.clone().unwrap_or(ty.clone());
                for child in children {
                    if let Value::String(s) = child {
                        content = s.clone();
                        break; // Use first string child
                    }
                }
                
                write!(f, "[{} {} '{}' ({} children)]", style, ty, content, count)
            }
            Value::Nil => write!(f, "nil"),
        }
    }
}

/// シンボルテーブル
/// 変数の初登場時に自動登録し、O(1)でアクセス可能
pub struct SymbolTable {
    pub symbols: HashMap<String, Value>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
        }
    }

    /// シンボルを登録（存在しなければ新規作成）
    pub fn register(&mut self, name: &str, value: Value) {
        self.symbols.insert(name.to_string(), value);
    }

    /// シンボルを参照
    pub fn lookup(&self, name: &str) -> Option<&Value> {
        self.symbols.get(name)
    }

    /// シンボルが存在するか確認
    #[allow(dead_code)]
    pub fn contains(&self, name: &str) -> bool {
        self.symbols.contains_key(name)
    }

    /// シンボルの値を更新
    pub fn update(&mut self, name: &str, value: Value) -> bool {
        if self.symbols.contains_key(name) {
            self.symbols.insert(name.to_string(), value);
            true
        } else {
            false
        }
    }

    /// シンボルの値を取得してクローン
    pub fn get_value(&self, name: &str) -> Value {
        self.symbols.get(name).cloned().unwrap_or(Value::Nil)
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_lookup() {
        let mut table = SymbolTable::new();
        table.register("X", Value::Number(10.0));
        
        let value = table.lookup("X").unwrap();
        match value {
            Value::Number(n) => assert_eq!(*n, 10.0),
            _ => panic!("Expected number"),
        }
    }

    #[test]
    fn test_update() {
        let mut table = SymbolTable::new();
        table.register("X", Value::Number(10.0));
        table.update("X", Value::Number(20.0));
        
        let value = table.lookup("X").unwrap();
        match value {
            Value::Number(n) => assert_eq!(*n, 20.0),
            _ => panic!("Expected number"),
        }
    }

    #[test]
    fn test_contains() {
        let mut table = SymbolTable::new();
        assert!(!table.contains("X"));
        table.register("X", Value::Number(10.0));
        assert!(table.contains("X"));
    }
}
