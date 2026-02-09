//! AGN Web Generator - WebAssembly用プロジェクト生成器
//! AGN ASTをRust + wasm-bindgenコードにトランスパイルし、ビルド環境を構築する

use crate::parser::{Expr, Program, Statement};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct WebGenerator {
    output_dir: PathBuf,
}

impl WebGenerator {
    pub fn new<P: AsRef<Path>>(output_dir: P) -> Self {
        Self {
            output_dir: output_dir.as_ref().to_path_buf(),
        }
    }

    /// Wasmプロジェクトを生成してビルドする
    pub fn generate_and_build(&self, program: &Program) -> Result<(), String> {
        self.setup_project_structure()?;
        self.generate_cargo_toml()?;
        self.generate_rust_code(program)?;
        self.generate_index_html()?;
        self.build_wasm()?;
        Ok(())
    }

    /// プロジェクトディレクトリ構造を作成
    fn setup_project_structure(&self) -> Result<(), String> {
        if !self.output_dir.exists() {
            fs::create_dir_all(&self.output_dir)
                .map_err(|e| format!("Failed to create output directory: {}", e))?;
        }
        
        let src_dir = self.output_dir.join("src");
        if !src_dir.exists() {
            fs::create_dir_all(&src_dir)
                .map_err(|e| format!("Failed to create src directory: {}", e))?;
        }
        
        Ok(())
    }

    /// Cargo.tomlを生成
    fn generate_cargo_toml(&self) -> Result<(), String> {
        let content = r#"[package]
name = "agn-web"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
wasm-bindgen = "0.2"
"#;
        
        let path = self.output_dir.join("Cargo.toml");
        let mut file = fs::File::create(path)
            .map_err(|e| format!("Failed to create Cargo.toml: {}", e))?;
        file.write_all(content.as_bytes())
            .map_err(|e| format!("Failed to write Cargo.toml: {}", e))?;
            
        Ok(())
    }

    /// Rustコード (lib.rs) を生成
    fn generate_rust_code(&self, program: &Program) -> Result<(), String> {
        let mut rs = String::new();
        
        // Header
        rs.push_str("use wasm_bindgen::prelude::*;\n\n");
        rs.push_str("#[wasm_bindgen]\n");
        rs.push_str("extern \"C\" {\n");
        rs.push_str("    #[wasm_bindgen(js_namespace = console)]\n");
        rs.push_str("    fn log(s: &str);\n\n");
        rs.push_str("    #[wasm_bindgen(js_namespace = window)]\n");
        rs.push_str("    fn updateScreen(s: &str);\n");
        rs.push_str("}\n\n");
        
        // Main function
        rs.push_str("#[wasm_bindgen(start)]\n");
        rs.push_str("pub fn main() {\n");
        rs.push_str("    // Generated from AGN source\n");
        
        // Transpile parsing statements
        for stmt in &program.statements {
            rs.push_str(&self.transpile_statement(stmt));
        }
        
        rs.push_str("}\n");
        
        let path = self.output_dir.join("src").join("lib.rs");
        let mut file = fs::File::create(path)
            .map_err(|e| format!("Failed to create lib.rs: {}", e))?;
        file.write_all(rs.as_bytes())
            .map_err(|e| format!("Failed to write lib.rs: {}", e))?;
            
        Ok(())
    }

    /// 文をRustコードにトランスパイル
    fn transpile_statement(&self, stmt: &Statement) -> String {
        match stmt {
            Statement::ScreenOp { operand } => {
                let val = self.transpile_expr_value(operand);
                // Send value as is (could be JSON string or primitive)
                format!("    updateScreen(&format!(\"{{}}\", {}));\n", val)
            }
            Statement::UnaryOp { operand, verb } => {
                if verb == "表示する" {
                    let val = self.transpile_expr_value(operand);
                    // Use updateScreen for "表示する" to support rich content
                    format!("    updateScreen(&format!(\"{{}}\", {}));\n", val)
                } else if verb == "要約する" || verb == "翻訳する" {
                    // WasmでのAI実行は現状プレースホルダー
                    let val = self.transpile_expr_value(operand);
                    format!("    // AI verb {} for {} (Not fully supported in Wasm yet)\n", verb, val)
                } else {
                    String::new()
                }
            }
            Statement::Block { body, .. } => {
                // For Web, block structure might map to DOM nesting.
                // For now, just transpile body flatly or ignore structure?
                // Phase 10 implements Layout. Web Generator uses JSON.
                // JSON is mostly "commands".
                // We should process body.
                let mut s = String::new();
                s.push_str("    // Block statement (transpiling body)\n");
                for stmt in body {
                    s.push_str(&self.transpile_statement(stmt));
                }
                s
            }
            Statement::Layout { .. } => {
                String::from("    // Layout command not yet transpiled for Web\n")
            }
            Statement::Assignment { target, value } => {
                if let Expr::Variable(name) = target {
                    let val = self.transpile_expr_value(value);
                    format!("    let {} = {};\n", name, val)
                } else {
                    String::from("    // Complex assignment not supported in Web transpiler yet\n")
                }
            }
            Statement::LoadAsset { target, path } => {
                if let Expr::Variable(name) = target {
                    let path_val = self.transpile_expr_value(path);
                    format!("    let {} = format!(\"{{\\\"type\\\":\\\"image\\\", \\\"src\\\":\\\"{{}}\\\"}}\", {});\n", name, path_val)
                } else {
                    String::from("    // Complex LoadAsset not supported in Web transpiler yet\n")
                }
            }
            Statement::ComponentDefine { target, style, component } => {
                 if let Expr::Variable(name) = target {
                     format!("    let {} = format!(\"{{\\\"type\\\":\\\"component\\\", \\\"style\\\":\\\"{}\\\", \\\"ty\\\":\\\"{}\\\"}}\");\n", name, style, component)
                 } else {
                     String::from("    // Complex ComponentDefine not supported in Web transpiler yet\n")
                 }
            }
            Statement::BinaryOp { target, operand, verb } => {
                if let Expr::Variable(name) = target {
                    let op_char = match verb.as_str() {
                        "足す" => "+",
                        "引く" => "-",
                        "掛ける" => "*",
                        "割る" => "/",
                        _ => "+",
                    };
                    let val = self.transpile_expr_value(operand);
                    format!("    let {} = {} {} {};\n", name, name, op_char, val)
                } else {
                    String::from("    // Complex BinaryOp not supported in Web transpiler yet\n")
                }
            }
            Statement::EventHandler { target, event, body } => {
                let mut s = String::new();
                let target_name = if let Expr::Variable(n) = target { n.clone() } else { format!("{:?}", target) };
                s.push_str(&format!("    // Event: on {} {}\n", target_name, event));
                for stmt in body {
                     s.push_str(&format!("    //   Body: {:?}\n", stmt));
                }
                s
            }
            Statement::AiOp { result: _, input: _, verb: _, options: _ } => {
                 String::from("    // AiOp (Complex Assignment) not fully supported in Wasm yet\n")
            }
            Statement::ActionCall { name, .. } => {
                 format!("    // Action call: {} (Wasm stub)\n", name)
            }
            Statement::ReturnStatement { .. } => {
                 String::from("    // Return statement\n")
            }
            _ => format!("    // Unsupported statement in Wasm: {:?}\n", stmt),
        }
    }

    fn transpile_expr_value(&self, expr: &Expr) -> String {
        match expr {
            Expr::Number(n) => format!("{:.1}", n),
            Expr::String(s) => format!("\"{}\"", s),
            Expr::Variable(name) => name.clone(),
            // Eeyo: 空間・時間リテラル
            Expr::Distance { value, unit } => format!("\"{:.1}{}\"", value, unit),
            Expr::Duration { value, unit } => format!("\"{:.1}{}\"", value, unit),
            // AGN 2.0: Property Access (Stub)
            Expr::PropertyAccess { .. } => String::from("\"[PropertyAccess Stub]\""),
            Expr::Bond(_, _) => String::from("\"[Bond Stub]\""),
            Expr::Call { name, .. } => format!("\"[Call Stub: {}]\"", name),
        }
    }

    /// index.htmlを生成
    fn generate_index_html(&self) -> Result<(), String> {
        let content = r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>AGN Web App</title>
    <style>
        body { font-family: 'Helvetica Neue', Arial, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; background: #f0f2f5; color: #1c1e21; }
        h1 { color: #1877f2; }
        #agn-screen { 
            border: none; 
            border-radius: 12px;
            padding: 24px; 
            min-height: 200px;
            background: #ffffff;
            margin-top: 20px;
            box-shadow: 0 4px 12px rgba(0,0,0,0.1);
        }
        .agn-output-item {
            margin-bottom: 12px;
            padding: 12px 16px;
            background: #f0f2f5;
            border-radius: 8px;
            font-size: 16px;
        }
        .agn-image {
            max-width: 100%;
            border-radius: 8px;
            box-shadow: 0 2px 8px rgba(0,0,0,0.1);
        }
        .agn-component {
            padding: 10px 20px;
            border: none;
            border-radius: 6px;
            font-size: 16px;
            cursor: pointer;
            transition: background 0.2s;
        }
        /* Style Mapping (declarative to CSS) */
        .style-blue { background-color: #1877f2; color: white; }
        .style-blue:hover { background-color: #166fe5; }
        .style-red { background-color: #ff3b30; color: white; }
        
        /* Default Button */
        .type-button { } 
    </style>
</head>
<body>
    <h1>AGN Screen Output</h1>
    <div id="agn-screen"></div>
    <script type="module">
        // Bridge function called from Rust
        window.updateScreen = (content) => {
            const el = document.getElementById('agn-screen');
            
            try {
                // Try to parse as JSON protocol
                const data = JSON.parse(content);
                
                if (data.type === 'image') {
                    const img = document.createElement('img');
                    img.src = data.src;
                    img.className = 'agn-output-item agn-image';
                    el.appendChild(img);
                } else if (data.type === 'component') {
                    const comp = document.createElement('button'); // Default to button for now
                    comp.className = `agn-output-item agn-component style-${data.style.toLowerCase()} type-${data.ty.toLowerCase()}`;
                    comp.textContent = data.ty; // Use type name as label for now
                    el.appendChild(comp);
                } else {
                    throw new Error("Unknown type");
                }
            } catch (e) {
                // Fallback to text
                const div = document.createElement('div');
                div.className = 'agn-output-item';
                div.textContent = content;
                el.appendChild(div);
            }
        };

        import init from './pkg/agn_web.js';
        
        async function run() {
            await init();
            console.log("AGN Wasm module initialized");
        }
        
        run();
    </script>
</body>
</html>"#;

        let path = self.output_dir.join("index.html");
        let mut file = fs::File::create(path)
            .map_err(|e| format!("Failed to create index.html: {}", e))?;
        file.write_all(content.as_bytes())
            .map_err(|e| format!("Failed to write index.html: {}", e))?;
            
        Ok(())
    }

    /// wasm-packを実行
    fn build_wasm(&self) -> Result<(), String> {
        println!("Building Wasm project in {:?}...", self.output_dir);
        
        let status = Command::new("wasm-pack")
            .arg("build")
            .arg("--target")
            .arg("web")
            .current_dir(&self.output_dir)
            .status()
            .map_err(|e| format!("Failed to run wasm-pack: {}", e))?;
            
        if !status.success() {
            return Err("wasm-pack build failed".to_string());
        }
        
        Ok(())
    }
}
