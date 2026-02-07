//! AGN - Antigravity-Native Language
//! 日本語ネイティブ文法の次世代プログラミング言語

mod lexer;
mod parser;
mod symbol_table;
mod interpreter;
mod normalizer;
mod type_inferencer;
mod ai_analyzer;
mod codegen;
mod compiler;
mod memory;
mod ai_runtime;
mod web_generator;
mod native_window;
mod graphics;
// Eeyo: P2P通信層
mod p2p;

use std::env;
use std::fs;
use std::time::Instant;

use lexer::Lexer;
use parser::Parser;
use interpreter::Interpreter;
use normalizer::Normalizer;
use type_inferencer::TypeInferencer;
use compiler::Compiler;
use memory::MemoryManager;

fn print_usage() {
    println!("Usage: agn [OPTIONS] [FILE]");
    println!();
    println!("Options:");
    println!("  --compile, -c    Compile to native binary");
    println!("  --run-compiled   Compile and run the binary");
    println!("  --emit-ir        Output LLVM IR only");
    println!("  --verbose, -v    Show detailed output");
    println!("  --tokens         Show tokens");
    println!("  --ast            Show AST");
    println!("  --types          Show type inference");
    println!("  --benchmark      Run benchmark comparison");
    println!("  --help, -h       Show this help");
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    
    // ヘルプ
    if args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        print_usage();
        return;
    }
    
    // フラグの解析
    let verbose = args.contains(&"--verbose".to_string()) || args.contains(&"-v".to_string());
    let show_tokens = args.contains(&"--tokens".to_string());
    let show_ast = args.contains(&"--ast".to_string());
    let show_types = args.contains(&"--types".to_string()) || verbose;
    let compile_mode = args.contains(&"--compile".to_string()) || args.contains(&"-c".to_string());
    let run_compiled = args.contains(&"--run-compiled".to_string());
    let emit_ir = args.contains(&"--emit-ir".to_string());
    let benchmark = args.contains(&"--benchmark".to_string());
    
    // ターゲット指定
    let target = if args.contains(&"--target".to_string()) {
        let idx = args.iter().position(|r| r == "--target").unwrap();
        if idx + 1 < args.len() {
            match args[idx+1].as_str() {
                "wasm" => compiler::Target::Wasm,
                "native-window" => compiler::Target::NativeWindow,
                _ => compiler::Target::Native,
            }
        } else {
            compiler::Target::Native
        }
    } else {
        compiler::Target::Native
    };
    
    // ソースファイルを探す
    let source_file = args.iter()
        .skip(1)
        .find(|arg| !arg.starts_with('-'))
        .cloned();
    
    let code = if let Some(ref file) = source_file {
        match fs::read_to_string(file) {
            Ok(content) => content,
            Err(e) => {
                eprintln!("Error reading file '{}': {}", file, e);
                return;
            }
        }
    } else {
        // デフォルトのテストコード
        r#"X は 10 だ
X に 5 を 足す
X を 並列で 表示する
"計算完了" を 表示する"#.to_string()
    };

    println!("=== AGN (Antigravity-Native) Phase 3 ===\n");

    // 1. 正規化
    let normalizer = Normalizer::new();
    let (normalized_code, corrections) = normalizer.normalize(&code);
    
    if !corrections.is_empty() && (verbose || !compile_mode) {
        println!("{}", normalizer.format_corrections(&corrections));
    }

    // コンパイルモード
    if compile_mode || run_compiled || emit_ir || benchmark || target == compiler::Target::Wasm {
        let output_name = source_file
            .as_ref()
            .map(|f| {
                std::path::Path::new(f)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("program")
                    .to_string()
            })
            .unwrap_or_else(|| "program".to_string());
        
        let output_dir = std::path::Path::new("./output");
        let mut compiler_instance = Compiler::new(output_dir);
        compiler_instance.set_verbose(verbose);
        compiler_instance.set_target(target.clone());
        
        match compiler_instance.compile(&normalized_code, &output_name) {
            Ok(result) => {
                println!("=== Compilation Successful ===");
                
                if target == compiler::Target::Wasm {
                    println!("  Wasm Project: {}", result.ir_path.parent().unwrap().display());
                    println!("  Artifact: {}", result.binary_path.display());
                    println!("\n[INFO] To run the Wasm app:");
                    println!("  cd {} && python3 -m http.server 8000", output_dir.display());
                    return;
                }

                println!("  IR: {}", result.ir_path.display());
                println!("  Binary: {}", result.binary_path.display());
                
                if emit_ir || verbose {
                    println!("\n=== LLVM IR ===");
                    println!("{}", result.ir_content);
                }
                
                if run_compiled || benchmark {
                    println!("\n=== Native Execution ===");
                    let native_start = Instant::now();
                    match result.run() {
                        Ok(output) => {
                            print!("{}", output);
                            let native_duration = native_start.elapsed();
                            print!("{}", output);
                            let native_duration = native_start.elapsed();
                            
                            if benchmark {
                                // インタプリタでも実行して比較
                                println!("\n=== Interpreter Execution ===");
                                let mut lexer = Lexer::new(&normalized_code);
                                let tokens = lexer.tokenize();
                                let mut parser = Parser::new(tokens);
                                
                                if let Ok(program) = parser.parse() {
                                    let interp_start = Instant::now();
                                    let interpreter = Interpreter::new();
                                    interpreter.execute(&program).await;
                                    let interp_duration = interp_start.elapsed();
                                    
                                    println!("\n=== Benchmark Results ===");
                                    println!("  Native:      {:?}", native_duration);
                                    println!("  Interpreter: {:?}", interp_duration);
                                    
                                    if native_duration < interp_duration {
                                        let speedup = interp_duration.as_nanos() as f64 
                                            / native_duration.as_nanos() as f64;
                                        println!("  Speedup:     {:.2}x faster", speedup);
                                    }
                                }
                            }
                        }
                        Err(e) => eprintln!("Execution error: {}", e),
                    }
                }
            }
            Err(e) => {
                eprintln!("Compile error: {}", e);
                return;
            }
        }
        
        return;
    }

    // インタプリタモード
    // 2. 字句解析
    let mut lexer = Lexer::new(&normalized_code);
    let tokens = lexer.tokenize();
    
    if show_tokens || verbose {
        println!("=== Tokens ===");
        for token in &tokens {
            println!("  {:?}", token);
        }
        println!();
    }

    // 3. 構文解析
    let mut parser = Parser::new(tokens);
    match parser.parse() {
        Ok(program) => {
            if show_ast || verbose {
                println!("=== AST ===");
                for stmt in &program.statements {
                    println!("  {:?}", stmt);
                }
                println!();
            }

            // 4. 型推論
            let inferencer = TypeInferencer::new();
            let type_result = inferencer.infer(&program);
            
            if show_types {
                println!("{}", type_result.to_human_readable());
            }

            // 5. メモリ分析
            if verbose {
                let mut mm = MemoryManager::new();
                mm.analyze(&type_result);
                println!("=== Memory Analysis ===");
                println!("  {}", mm.get_stats());
                println!();
            }

            // 6. 実行
            if target == compiler::Target::NativeWindow {
                println!("=== Native Window Mode ===");
                let (tx, rx) = std::sync::mpsc::channel();
                
                {
                    let mut guard = interpreter::SCREEN_CHANNEL.lock().unwrap();
                    *guard = Some(tx);
                }
                
                let program_clone = program.clone();
                let symbol_table = std::sync::Arc::new(std::sync::Mutex::new(symbol_table::SymbolTable::new()));
                let symbol_table_for_window = symbol_table.clone();
                
                std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new().unwrap();
                    rt.block_on(async {
                       let interpreter = Interpreter::with_symbol_table(symbol_table);
                       interpreter.execute(&program_clone).await;
                    });
                });
                
                // Convert Tokio Mutex to Std Mutex? 
                // symbol_table is Arc<tokio::Mutex>. native_window expects Arc<std::Mutex>?
                // Actually they should match. 
                // native_window.rs uses std::sync::Mutex in my previous edit?
                // Let's check native_window import.
                // Wait, Interpreter uses tokio::sync::Mutex.
                // Native Window loop is synchronous (winit).
                // Blocking on tokio mutex in main loop might be okay if we use blocking_lock() but it's async mutex.
                // Better use std::sync::Mutex for everything if possible, or wrap properly.
                // Interpreter relies on async eval... so it needs async lock?
                // Actually Interpreter uses tokio::sync::Mutex.
                // If I change Interpreter to use std::sync::Mutex, async eval_expr will block thread.
                // Since Interpreter is async, it should probably keep using async mutex.
                // Native Window needs to read it.
                // We can use std::sync::Mutex for SymbolTable, and wrap it in Arc.
                // In async context, we can use std::Mutex if we don't hold it across await points.
                // eval_expr holds it briefly.
                // execute_statements holds it briefly.
                // So std::sync::Mutex is fine strictly speaking if no await inside lock.
                // Let's check interpreter.rs imports.
                // interpreter.rs: use tokio::sync::Mutex; 
                // I should change interpreter.rs to use std::sync::Mutex or RwLock?
                // Or I can use a bridge.
                // For simplicity, let's keep tokio::Mutex and use `try_lock()` or `blocking_lock()` in native window?
                // Native window is not async.
                // `tokio::sync::Mutex` has `blocking_lock()`? No, it's async specialized.
                // It has `blocking_lock()` in recent versions if feature enabled?
                // Safer to swap Interpreter to use std::sync::Mutex.
                // Most simple interpreters don't need async locks unless they do IO inside lock.
                // Let's update main.rs assuming I fix types later or now.
                // I'll update main.rs to use the same type as interpreter expects.
                native_window::run_native_window(rx, symbol_table_for_window);
                return;
            }

            println!("=== Output ===");
            let interpreter = Interpreter::new();
            interpreter.execute(&program).await;
        }
        Err(e) => {
            eprintln!("Parse error: {}", e);
            eprintln!("\nOriginal code:");
            for (i, line) in code.lines().enumerate() {
                eprintln!("  {}: {}", i + 1, line);
            }
            eprintln!("\nNormalized code:");
            for (i, line) in normalized_code.lines().enumerate() {
                eprintln!("  {}: {}", i + 1, line);
            }
        }
    }
}
