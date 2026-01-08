//! AGN Code Generator - LLVM IR生成器
//! ASTからLLVM IRを生成する

use crate::parser::{Expr, Program, Statement};
use crate::type_inferencer::{InferredType, TypeInferenceResult};

pub struct CodeGenerator {
    temp_counter: usize,
    string_constants: Vec<(String, String)>, // (name, content)
}

impl CodeGenerator {
    pub fn new() -> Self {
        Self {
            temp_counter: 0,
            string_constants: Vec::new(),
        }
    }

    fn next_temp(&mut self) -> String {
        self.temp_counter += 1;
        format!("%t{}", self.temp_counter)
    }

    fn add_string_constant(&mut self, content: &str) -> String {
        let name = format!("@.str.{}", self.string_constants.len());
        self.string_constants.push((name.clone(), content.to_string()));
        name
    }

    /// ASTからLLVM IRを生成
    pub fn generate(&mut self, program: &Program, type_info: &TypeInferenceResult) -> String {
        let mut ir = String::new();

        // Header
        ir.push_str("; ModuleID = 'agn_module'\n");
        ir.push_str("source_filename = \"agn_source\"\n");
        ir.push_str("target datalayout = \"e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128\"\n");
        ir.push_str("target triple = \"x86_64-pc-linux-gnu\"\n\n");

        // External Declarations
        ir.push_str("declare i32 @printf(i8*, ...)\n\n");

        // Constant format strings
        ir.push_str("@.str.fmt.int = private unnamed_addr constant [6 x i8] c\"%.1f\\0A\\00\", align 1\n");
        ir.push_str("@.str.fmt.str = private unnamed_addr constant [4 x i8] c\"%s\\0A\\00\", align 1\n\n");

        // String constants place holder (will be populated during generation)
        // Note: Standard LLVM structure usually puts constants at top.
        // For simplicity in single pass, we might have issues if we strictly follow order.
        // But let's verify if we can buffer the body.
        
        let mut body_ir = String::new();
        body_ir.push_str("define i32 @main() {\n");
        body_ir.push_str("entry:\n");

        // 変数の確保 (alloca)
        for var in &type_info.variables {
            if var.inferred_type == InferredType::Number || var.inferred_type == InferredType::Unknown {
                body_ir.push_str(&format!("    %{} = alloca double, align 8\n", var.name));
            } else if var.inferred_type == InferredType::String {
                // String pointer
                // NOTE: Simply using i8* (pointer) for string variables
                body_ir.push_str(&format!("    %{} = alloca i8*, align 8\n", var.name));
            }
        }
        body_ir.push('\n');

        // 文の生成
        for stmt in &program.statements {
            body_ir.push_str(&self.emit_statement(stmt));
        }

        body_ir.push_str("    ret i32 0\n");
        body_ir.push_str("}\n");

        // Add collected constants to header
        for (name, content) in &self.string_constants {
            let escaped = escape_string(content);
            let len = content.len() + 1; // +1 for null terminator
            ir.push_str(&format!("{} = private unnamed_addr constant [{} x i8] c\"{}{}\\00\", align 1\n", 
                name, len, escaped, if  content.is_empty() { "" } else { "" })); 
                // fix: escape_string logic might be intricate, simplified for now
        }
        ir.push('\n');

        ir.push_str(&body_ir);

        ir
    }

    fn emit_statement(&mut self, stmt: &Statement) -> String {
        match stmt {
            Statement::EventHandler { .. } => {
                String::from("    ; Event Handler not implemented in LLVM backend yet\n")
            }
            Statement::Block { .. } | Statement::Layout { .. } => {
                String::from("    ; UI Block/Layout not implemented in LLVM backend yet\n")
            }
            Statement::Assignment { name, value } => {
                self.emit_assignment(name, value)
            }
            Statement::BinaryOp { target, operand, verb } => {
                self.emit_binary_op(target, operand, verb)
            }
            Statement::UnaryOp { operand, verb } => {
                self.emit_unary_op(operand, verb)
            }
            Statement::LoadAsset { .. } | Statement::ComponentDefine { .. } => {
                String::from("    ; LoadAsset/ComponentDefine not implemented in LLVM backend yet\n")
            }
            _ => String::from("    ; Unsupported statement\n"),
        }
    }

    /// 代入文を生成
    fn emit_assignment(&mut self, name: &str, value: &Expr) -> String {
        let mut ir = String::new();

        match value {
            Expr::Number(n) => {
                let formatted = if n.fract() == 0.0 {
                    format!("{:.1}", n)
                } else {
                    format!("{}", n)
                };
                ir.push_str(&format!("    store double {}, double* %{}, align 8\n", 
                    formatted, name));
            }
            Expr::String(s) => {
                // 文字列は別途処理が必要（Phase 4で拡張）
                let const_name = self.add_string_constant(s);
                ir.push_str(&format!("    ; String assignment: {} = \"{}\"\n", name, s));
                // ir.push_str(&format!("    store i8* getelementptr ... (TODO)\n")); 
            }
            Expr::Variable(var_name) => {
                let temp = self.next_temp();
                ir.push_str(&format!("    {} = load double, double* %{}, align 8\n", 
                    temp, var_name));
                ir.push_str(&format!("    store double {}, double* %{}, align 8\n", 
                    temp, name));
            }
        }

        ir
    }

    /// 二項演算を生成
    fn emit_binary_op(&mut self, target: &str, operand: &Expr, verb: &str) -> String {
        let mut ir = String::new();

        // ターゲットの現在の値をロード
        let target_val = self.next_temp();
        ir.push_str(&format!("    {} = load double, double* %{}, align 8\n", 
            target_val, target));

        // オペランドの値を取得
        let operand_val = match operand {
            Expr::Number(n) => {
                if n.fract() == 0.0 {
                    format!("{:.1}", n)
                } else {
                    format!("{}", n)
                }
            }
            Expr::Variable(var_name) => {
                let temp = self.next_temp();
                ir.push_str(&format!("    {} = load double, double* %{}, align 8\n", 
                    temp, var_name));
                temp
            }
            Expr::String(_) => {
                ir.push_str("    ; Warning: String operand in binary op not supported\n");
                "0.0".to_string()
            }
        };

        // 演算を実行
        let result = self.next_temp();
        let op = match verb {
            "足す" => "fadd",
            "引く" => "fsub",
            "掛ける" => "fmul",
            "割る" => "fdiv",
            _ => {
                ir.push_str(&format!("    ; Unknown verb: {}\n", verb));
                "fadd" // デフォルト
            }
        };
        
        ir.push_str(&format!("    {} = {} double {}, {}\n", 
            result, op, target_val, operand_val));
        
        // 結果を格納
        ir.push_str(&format!("    store double {}, double* %{}, align 8\n", 
            result, target));

        ir
    }

    /// 単項演算（表示など）を生成
    fn emit_unary_op(&mut self, operand: &Expr, verb: &str) -> String {
        let mut ir = String::new();

        match verb {
            "表示する" => {
                match operand {
                    Expr::Number(n) => {
                        // 整数として表示
                        let formatted = if n.fract() == 0.0 {
                            format!("{:.1}", n)
                        } else {
                            format!("{}", n)
                        };
                        let fmt_ptr = self.next_temp();
                        ir.push_str(&format!(
                            "    {} = getelementptr [6 x i8], [6 x i8]* @.str.fmt.int, i64 0, i64 0\n",
                            fmt_ptr
                        ));
                        ir.push_str(&format!(
                            "    call i32 (i8*, ...) @printf(i8* {}, double {})\n",
                            fmt_ptr, formatted
                        ));
                    }
                    Expr::Variable(var_name) => {
                        let val = self.next_temp();
                        ir.push_str(&format!("    {} = load double, double* %{}, align 8\n", 
                            val, var_name));
                        let fmt_ptr = self.next_temp();
                        ir.push_str(&format!(
                            "    {} = getelementptr [6 x i8], [6 x i8]* @.str.fmt.int, i64 0, i64 0\n",
                            fmt_ptr
                        ));
                        ir.push_str(&format!(
                            "    call i32 (i8*, ...) @printf(i8* {}, double {})\n",
                            fmt_ptr, val
                        ));
                    }
                    Expr::String(s) => {
                        let const_name = self.add_string_constant(s);
                        let _len = s.len() + 1;
                        let str_ptr = self.next_temp();
                        // String printing LLVM IR is tricky for placeholder, just assume constant access
                        ir.push_str(&format!(
                            "    {} = getelementptr [{} x i8], [{} x i8]* {}, i64 0, i64 0\n",
                            str_ptr, s.len() + 1, s.len() + 1, const_name
                        ));
                        let fmt_ptr = self.next_temp();
                        ir.push_str(&format!(
                            "    {} = getelementptr [4 x i8], [4 x i8]* @.str.fmt.str, i64 0, i64 0\n",
                            fmt_ptr
                        ));
                        ir.push_str(&format!(
                            "    call i32 (i8*, ...) @printf(i8* {}, i8* {})\n",
                            fmt_ptr, str_ptr
                        ));
                    }
                }
            }
            _ => {
                ir.push_str(&format!("    ; Unknown unary verb: {}\n", verb));
            }
        }

        ir
    }
}

impl Default for CodeGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// 文字列をLLVM IR用にエスケープ
fn escape_string(s: &str) -> String {
    let mut result = String::new();
    for c in s.chars() {
        match c {
            '\n' => result.push_str("\\0A"),
            '\r' => result.push_str("\\0D"),
            '\t' => result.push_str("\\09"),
            '\\' => result.push_str("\\5C"),
            '"' => result.push_str("\\22"),
            _ if c.is_ascii() => result.push(c),
            _ => {
                // UTF-8バイト列としてエスケープ
                for b in c.to_string().as_bytes() {
                    result.push_str(&format!("\\{:02X}", b));
                }
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::type_inferencer::TypeInferencer;

    #[test]
    fn test_simple_assignment() {
        let code = "X は 10 だ";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        let inferencer = TypeInferencer::new();
        let type_info = inferencer.infer(&program);
        
        let mut codegen = CodeGenerator::new();
        let ir = codegen.generate(&program, &type_info);
        
        assert!(ir.contains("define i32 @main()"));
        assert!(ir.contains("alloca double"));
        assert!(ir.contains("store double"));
    }

    #[test]
    fn test_binary_op() {
        let code = "X は 10 だ\nX に 5 を 足す";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        let inferencer = TypeInferencer::new();
        let type_info = inferencer.infer(&program);
        
        let mut codegen = CodeGenerator::new();
        let ir = codegen.generate(&program, &type_info);
        
        assert!(ir.contains("fadd double"));
    }

    #[test]
    fn test_print() {
        let code = "X は 10 だ\nX を 表示する";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        let inferencer = TypeInferencer::new();
        let type_info = inferencer.infer(&program);
        
        let mut codegen = CodeGenerator::new();
        let ir = codegen.generate(&program, &type_info);
        
        assert!(ir.contains("call i32 (i8*, ...) @printf"));
    }
}
