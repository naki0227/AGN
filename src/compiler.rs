//! AGN Compiler - コンパイルパイプライン
//! AGNソースコードからネイティブバイナリを生成する

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::codegen::CodeGenerator;
use crate::lexer::Lexer;
use crate::normalizer::Normalizer;
use crate::parser::Parser;
use crate::type_inferencer::TypeInferencer;

/// コンパイルターゲット
#[derive(Debug, Clone, PartialEq)]
pub enum Target {
    Native,
    Wasm,
    NativeWindow,
}

/// コンパイルエラー
#[derive(Debug)]
pub enum CompileError {
    IoError(io::Error),
    ParseError(String),
    ClangError(String),
    ClangNotFound,
    WebGeneratorError(String),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::IoError(e) => write!(f, "IO error: {}", e),
            CompileError::ParseError(e) => write!(f, "Parse error: {}", e),
            CompileError::ClangError(e) => write!(f, "Clang error: {}", e),
            CompileError::ClangNotFound => write!(f, "clang not found in PATH"),
            CompileError::WebGeneratorError(e) => write!(f, "Web Generator error: {}", e),
        }
    }
}

impl From<io::Error> for CompileError {
    fn from(err: io::Error) -> Self {
        CompileError::IoError(err)
    }
}

/// コンパイラ
pub struct Compiler {
    output_dir: PathBuf,
    optimization_level: u8,
    verbose: bool,
    target: Target,
}

impl Compiler {
    pub fn new<P: AsRef<Path>>(output_dir: P) -> Self {
        Self {
            output_dir: output_dir.as_ref().to_path_buf(),
            optimization_level: 2,
            verbose: false,
            target: Target::Native,
        }
    }
    
    pub fn set_target(&mut self, target: Target) {
        self.target = target;
    }

    #[allow(dead_code)]
    pub fn set_verbose(&mut self, verbose: bool) {
        self.verbose = verbose;
    }

    #[allow(dead_code)]
    pub fn set_optimization(&mut self, level: u8) {
        self.optimization_level = level.min(3);
    }

    /// ソースコードをコンパイル
    pub fn compile(&self, source: &str, output_name: &str) -> Result<CompileResult, CompileError> {
        // 出力ディレクトリを作成
        fs::create_dir_all(&self.output_dir)?;

        // 1. 正規化
        let normalizer = Normalizer::new();
        let (normalized, _corrections) = normalizer.normalize(source);

        // 2. 字句解析
        let mut lexer = Lexer::new(&normalized);
        let tokens = lexer.tokenize();

        // 3. 構文解析
        let mut parser = Parser::new(tokens);
        let program = parser.parse().map_err(CompileError::ParseError)?;

        // 4. 型推論
        let inferencer = TypeInferencer::new();
        let type_info = inferencer.infer(&program);
        
        // ターゲットによる分岐
        if self.target == Target::Wasm {
            return self.compile_wasm(&program, output_name);
        }

        // 5. LLVM IR生成
        let mut codegen = CodeGenerator::new();
        let ir = codegen.generate(&program, &type_info);

        // 6. IRファイルを書き出し
        let ir_path = self.output_dir.join(format!("{}.ll", output_name));
        fs::write(&ir_path, &ir)?;

        if self.verbose {
            println!("[Compiler] Generated IR: {}", ir_path.display());
        }

        // 7. clangでコンパイル
        let binary_path = self.output_dir.join(output_name);
        self.invoke_clang(&ir_path, &binary_path)?;

        if self.verbose {
            println!("[Compiler] Generated binary: {}", binary_path.display());
        }

        Ok(CompileResult {
            ir_path,
            binary_path,
            ir_content: ir,
        })
    }
    
    /// Wasmコンパイル (トランスパイル + wasm-pack)
    fn compile_wasm(&self, program: &crate::parser::Program, output_name: &str) -> Result<CompileResult, CompileError> {
        use crate::web_generator::WebGenerator;
        
        let web_gen = WebGenerator::new(&self.output_dir);
        web_gen.generate_and_build(program).map_err(CompileError::WebGeneratorError)?;
        
        // Wasmの場合はバイナリパスなどは便宜上のものを返す
        let ir_path = self.output_dir.join("src/lib.rs");
        let binary_path = self.output_dir.join("pkg").join("agn_web_bg.wasm");
        
        Ok(CompileResult {
            ir_path,
            binary_path,
            ir_content: "// Transpiled to Rust + Wasm".to_string(),
        })
    }

    /// clangを呼び出してネイティブバイナリを生成
    fn invoke_clang(&self, ir_path: &Path, output_path: &Path) -> Result<(), CompileError> {
        let output = Command::new("clang")
            .arg(format!("-O{}", self.optimization_level))
            .arg("-Wno-override-module")
            .arg(ir_path)
            .arg("-o")
            .arg(output_path)
            .output();

        match output {
            Ok(result) => {
                if result.status.success() {
                    Ok(())
                } else {
                    let stderr = String::from_utf8_lossy(&result.stderr);
                    Err(CompileError::ClangError(stderr.to_string()))
                }
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                Err(CompileError::ClangNotFound)
            }
            Err(e) => Err(CompileError::IoError(e)),
        }
    }
}

/// コンパイル結果
pub struct CompileResult {
    pub ir_path: PathBuf,
    pub binary_path: PathBuf,
    pub ir_content: String,
}

impl CompileResult {
    /// 生成されたバイナリを実行
    pub fn run(&self) -> Result<String, io::Error> {
        let output = Command::new(&self.binary_path).output()?;
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        if !stderr.is_empty() {
            eprintln!("{}", stderr);
        }
        
        Ok(stdout.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;

    #[test]
    fn test_compile_simple() {
        let source = "X は 10 だ\nX を 表示する";
        let output_dir = temp_dir().join("agn_test");
        
        let compiler = Compiler::new(&output_dir);
        let result = compiler.compile(source, "test_simple");
        
        match result {
            Ok(res) => {
                assert!(res.ir_path.exists());
                assert!(res.binary_path.exists());
                assert!(res.ir_content.contains("define i32 @main()"));
                
                // クリーンアップ
                let _ = fs::remove_dir_all(&output_dir);
            }
            Err(CompileError::ClangNotFound) => {
                // clangがない環境ではスキップ
                println!("Skipping test: clang not found");
            }
            Err(e) => panic!("Compile failed: {}", e),
        }
    }
}
