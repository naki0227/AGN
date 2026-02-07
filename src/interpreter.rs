//! AGN Interpreter - インタプリタ
//! ASTを直接実行する（制御構文を含む）

use crate::parser::{Condition, Expr, Program, Statement};
use crate::symbol_table::{SymbolTable, Value};
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::sync::mpsc::Sender;
use web_time::{Duration, Instant};

use crate::graphics::animation::Animation;
// Eeyo: P2P通信層
use crate::p2p::{agn_spatial_search, agn_broadcast_beacon, agn_notify_peer};

#[derive(Debug, Clone)]
pub enum RuntimeMessage {
    String(String),
    Animate(Animation),
    RegisterEvent(String, String, Vec<Animation>),
    LoadImage(String, String),
}

pub static SCREEN_CHANNEL: StdMutex<Option<Sender<RuntimeMessage>>> = StdMutex::new(None);
use tokio::sync::Mutex;

#[cfg(not(target_arch = "wasm32"))]
fn spawn_async<F>(future: F)
where F: std::future::Future<Output = ()> + Send + 'static {
    tokio::spawn(future);
}

#[cfg(target_arch = "wasm32")]
fn spawn_async<F>(future: F)
where F: std::future::Future<Output = ()> + 'static {
    wasm_bindgen_futures::spawn_local(future);
}

#[derive(Clone)]
pub struct Interpreter {
    pub symbol_table: Arc<StdMutex<SymbolTable>>,
    pub context_stack: Arc<StdMutex<Vec<String>>>,
    pub event_handlers: Arc<StdMutex<std::collections::HashMap<(String, String), Vec<Statement>>>>,
}

 impl Interpreter {
    pub fn new() -> Self {
        Self {
            symbol_table: Arc::new(StdMutex::new(SymbolTable::new())),
            context_stack: Arc::new(StdMutex::new(Vec::new())),
            event_handlers: Arc::new(StdMutex::new(std::collections::HashMap::new())),
        }
    }

    pub fn with_symbol_table(symbol_table: Arc<StdMutex<SymbolTable>>) -> Self {
        Self {
            symbol_table,
            context_stack: Arc::new(StdMutex::new(Vec::new())),
            event_handlers: Arc::new(StdMutex::new(std::collections::HashMap::new())),
        }
    }

    async fn eval_expr(&self, expr: &Expr) -> Value {
        match expr {
            Expr::Number(n) => Value::Number(*n),
            Expr::String(s) => Value::String(s.clone()),
            Expr::Variable(name) => {
                let table = self.symbol_table.lock().unwrap();
                table.get_value(name)
            }
            // Eeyo: 空間・時間型
            Expr::Distance { value, unit } => {
                Value::String(format!("{}{}", value, unit))
            }
            Expr::Duration { value, unit } => {
                Value::String(format!("{}{}", value, unit))
            }
        }
    }

    async fn eval_condition(&self, condition: &Condition) -> bool {
        match condition {
            Condition::Equals(left, right) => {
                let left_val = self.eval_expr(left).await;
                let right_val = self.eval_expr(right).await;
                match (left_val, right_val) {
                    (Value::Number(a), Value::Number(b)) => a == b,
                    (Value::String(a), Value::String(b)) => a == b,
                    _ => false,
                }
            }
            Condition::GreaterThan(left, right) => {
                let left_val = self.eval_expr(left).await;
                let right_val = self.eval_expr(right).await;
                match (left_val, right_val) {
                    (Value::Number(a), Value::Number(b)) => a > b,
                    _ => false,
                }
            }
            Condition::LessThan(left, right) => {
                let left_val = self.eval_expr(left).await;
                let right_val = self.eval_expr(right).await;
                match (left_val, right_val) {
                    (Value::Number(a), Value::Number(b)) => a < b,
                    _ => false,
                }
            }
            // Eeyo: 空間条件（後方互換性のためにプレースホルダー）
            Condition::Nearer(_) | Condition::Farther(_) => {
                // TODO: P2Pレイヤーで実装予定
                log::warn!("[空間条件] Nearer/FartherはP2Pレイヤーで実装予定");
                false
            }
        }
    }

    pub async fn execute(&self, program: &Program) {
        self.execute_statements(&program.statements).await;
    }

    pub async fn execute_statements(&self, statements: &[Statement]) {
        // let mut handles = Vec::new();

        for stmt in statements {
            match stmt {
                Statement::Assignment { name, value } => {
                    let val = self.eval_expr(value).await;
                    let mut table = self.symbol_table.lock().unwrap();
                    table.register(name, val.clone());
                    
                    // Check context stack to see if we are inside a block (Implicit Child Addition)
                    // Only for UI-related values (String, Component, Image)
                    if matches!(val, Value::String(_) | Value::Component { .. } | Value::Image(_)) {
                         drop(table); // unlock value table before locking stack
                         
                         let stack = self.context_stack.lock().unwrap();
                         if let Some(parent_name) = stack.last() {
                             let mut table = self.symbol_table.lock().unwrap();
                             if let Some(Value::Component { children, .. }) = table.lookup(parent_name).cloned().as_mut() {
                                  // Update parent
                                  let parent_val = table.get_value(parent_name);
                                  if let Value::Component { style, ty, label, mut children, layout } = parent_val {
                                      children.push(val); // Add copy of value
                                      table.update(parent_name, Value::Component { style, ty, label, children, layout });
                                  }
                             }
                         }
                    }
                }
                
                Statement::ComponentDefine { target, style, component } => {
                     let mut table = self.symbol_table.lock().unwrap();

                     let comp_val = Value::Component { 
                         style: style.clone(), 
                         ty: component.clone(), 
                         label: Some(target.clone()),
                         children: Vec::new(),
                         layout: None,
                     };
                     
                     table.register(target, comp_val.clone());
                     
                     // Check context stack to see if we are inside a block
                     drop(table); // unlock value table before locking stack
                     
                     let stack = self.context_stack.lock().unwrap();
                     if let Some(parent_name) = stack.last() {
                         let mut table = self.symbol_table.lock().unwrap();
                         if let Some(Value::Component { children, .. }) = table.lookup(parent_name).cloned().as_mut() {
                              // We need to update the parent in the table
                              // But lookup returns reference or we clone it?
                              // clone() creates a copy. as_mut() on clone doesn't help.
                              // We need to get the value, modify it, and update it.
                              let parent_val = table.get_value(parent_name);
                              if let Value::Component { style, ty, label, mut children, layout } = parent_val {
                                  children.push(comp_val);
                                  table.update(parent_name, Value::Component { style, ty, label, children, layout });
                              }
                         }
                     }
                }
                Statement::Block { target, body } => {
                    // Push target to stack
                    {
                        let mut stack = self.context_stack.lock().unwrap();
                        stack.push(target.clone());
                    }
                    
                    // Execute body
                    Box::pin(self.execute_statements(&body)).await;
                    
                    // Pop stack
                    {
                        let mut stack = self.context_stack.lock().unwrap();
                        stack.pop();
                    }
                }
                Statement::Layout { target, direction } => {
                    // Set layout on target. 
                    // If target is "これら", use current context (parent).
                    let target_name = if target == "これら" {
                        let stack = self.context_stack.lock().unwrap();
                        if let Some(parent) = stack.last() {
                            parent.clone()
                        } else {
                            eprintln!("Error: 'これら' used outside of block");
                            continue;
                        }
                    } else {
                        target.clone()
                    };
                    
                    let dir_str = match direction {
                        crate::parser::LayoutDirection::Vertical => "vertical".to_string(),
                        crate::parser::LayoutDirection::Horizontal => "horizontal".to_string(),
                    };
                    
                    let mut table = self.symbol_table.lock().unwrap();
                    let val = table.get_value(&target_name);
                    if let Value::Component { style, ty, label, children, .. } = val {
                        table.update(&target_name, Value::Component { 
                            style, ty, label, children, layout: Some(dir_str) 
                        });
                    }
                }
                Statement::LoadAsset { target, path } => {
                     let path_val = self.eval_expr(path).await;
                     let path_str = match path_val {
                         Value::String(s) => s,
                         _ => {
                             eprintln!("Error: Asset path must be a string");
                             continue;
                         }
                     };
                     
                     println!("[System] Loading asset: {}", path_str);
                     // Send to Native Window
                     if let Ok(guard) = SCREEN_CHANNEL.lock() {
                        if let Some(tx) = &*guard {
                            let _ = tx.send(RuntimeMessage::LoadImage(target.clone(), path_str.clone()));
                        }
                     }
                     // Keep Value::Image for SymbolTable if needed
                     let val = Value::Image(path_str);
                     
                     let mut table = self.symbol_table.lock().unwrap();
                     table.register(target, val.clone());
                     
                     drop(table);
                     // (Optional: Update parent if needed - simpler to skip for now unless layout requires it)
                }
                Statement::BinaryOp { target, operand, verb } => {
                    let op_val = self.eval_expr(operand).await;
                    
                    // Handle "Screen" Display (e.g. "MainButton を 画面 に 表示する")
                    if (target == "Screen" || target == "Screen.Center") && verb == "表示する" {
                         log::info!("[Output] {}", op_val);
                         if let Ok(guard) = SCREEN_CHANNEL.lock() {
                            if let Some(tx) = &*guard {
                                let _ = tx.send(RuntimeMessage::String(op_val.to_string()));
                            }
                         }
                         continue;
                    }

                    let mut table = self.symbol_table.lock().unwrap();
                    
                    // Numeric Operations
                    if let Some(Value::Number(current)) = table.lookup(target).cloned() {
                        if let Value::Number(op_num) = op_val {
                             let result = match verb.as_str() {
                                "足す" => current + op_num,
                                "引く" => current - op_num,
                                "掛ける" => current * op_num,
                                "割る" => if op_num != 0.0 { current / op_num } else { current },
                                _ => current,
                             };
                             table.update(target, Value::Number(result));
                        }
                    }
                    // Component Operations (e.g. "つなぐ")
                    // Target "Card", Operand "Hello", Verb "つなぐ"
                    else if let Some(Value::Component { .. }) = table.lookup(target).cloned() {
                        if verb == "つなぐ" {
                            // Append child
                            // Need to mutate the component in the table
                            let parent_val = table.get_value(target);
                            if let Value::Component { style, ty, label, mut children, layout } = parent_val {
                                children.push(op_val.clone());
                                table.update(target, Value::Component { style, ty, label, children, layout });
                            }
                        }
                    }
                }
                Statement::UnaryOp { operand, verb } => {
                    let val = self.eval_expr(operand).await;
                    self.execute_verb(verb, val).await;
                }
                Statement::AsyncOp { operand, verb } => {
                    let val = self.eval_expr(operand).await;
                    let verb = verb.clone();
                    
                    let handle = spawn_async(async move {
                        execute_verb_static(&verb, val).await;
                    });
                    // handles.push(handle); // spawn_async returns () on wasm?
                    // Actually, we don't need to track handles for basic async op unless we join.
                    // For now, fire and forget.
                }
                Statement::IfStatement { condition, then_block, else_block } => {
                    let cond_result = self.eval_condition(condition).await;
                    if cond_result {
                        Box::pin(self.execute_statements(then_block)).await;
                    } else if let Some(else_stmts) = else_block {
                        Box::pin(self.execute_statements(else_stmts)).await;
                    }
                }
                Statement::RepeatStatement { count, body } => {
                    let count_val = self.eval_expr(count).await;
                    if let Value::Number(n) = count_val {
                        let iterations = n as usize;
                        for _ in 0..iterations {
                            Box::pin(self.execute_statements(body)).await;
                        }
                    }
                }
                Statement::AiOp { result, input, verb, options } => {
                    let input_val = self.eval_expr(input).await;
                    let input_str = match input_val {
                        Value::String(s) => s,
                        Value::Number(n) => n.to_string(),
                        _ => String::new(),
                    };
                    
                    let option_val = if let Some(opt_expr) = options {
                        let val = self.eval_expr(opt_expr).await;
                        Some(val.to_string())
                    } else {
                        None
                    };
                    
                    let runtime = crate::ai_runtime::AiRuntime::new();
                    match runtime.execute_verb(verb, &input_str, option_val).await {
                        Ok(ai_result) => {
                            log::info!("[AI] {} result: {}", verb, &ai_result);
                            let mut table = self.symbol_table.lock().unwrap();
                            table.register(result, Value::String(ai_result));
                        }
                        Err(e) => {
                            log::error!("[AI Error] {}: {}", verb, e);
                            let mut table = self.symbol_table.lock().unwrap();
                            table.register(result, Value::String(format!("[AI Error: {}]", e)));
                        }
                    }
                }
                Statement::ScreenOp { operand } => {
                    let val = self.eval_expr(operand).await;
                    log::info!("[Output] {}", val);
                    
                    if let Ok(guard) = SCREEN_CHANNEL.lock() {
                        if let Some(tx) = &*guard {
                            let _ = tx.send(RuntimeMessage::String(val.to_string()));
                        }
                    }
                }
                Statement::DelayStatement { duration, body } => {
                    let duration_val = self.eval_expr(duration).await;
                    if let Value::Number(secs) = duration_val {
                        // Async delay
                        let sleep_ms = (secs * 1000.0) as u64;
                        #[cfg(target_arch = "wasm32")]
                        {
                            // Use setTimeout via Promise in Wasm?
                            // Or gloo_timers? 
                            // Since we are async, we can use a helper.
                            // But wait, `std::thread::sleep` blocks. We need async sleep.
                            // `web_time` doesn't provide async sleep directly?
                            // We can use a simple JS promise wrapper or `gloo_timers::future::TimeoutFuture`.
                            // For now, let's use a simple spin or assumption that `tokio` isn't available in minimal wasm.
                            // Actually, I can use a helper function for async sleep.
                            crate::utils::sleep(sleep_ms).await;
                        }
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                             tokio::time::sleep(tokio::time::Duration::from_millis(sleep_ms)).await;
                        }
                        
                        Box::pin(self.execute_statements(body)).await;
                    }
                }
                Statement::AnimateStatement { duration, target, property, value } => {
                    let duration_val = self.eval_expr(duration).await;
                    let target_val = self.eval_expr(value).await;
                    
                    let duration_secs = match duration_val {
                        Value::Number(n) => n,
                        _ => 0.0,
                    };
                    
                    // Construct RuntimeMessage for Animation
                    // Frontend will handle the transition
                    let msg = format!("[Animation] Target: {}, Prop: {}, Value: {}, Duration: {}s", target, property, target_val, duration_secs);
                    log::info!("{}", msg);
                    
                    if let Ok(guard) = SCREEN_CHANNEL.lock() {
                        if let Some(tx) = &*guard {
                            // Send custom message or overload String/Event?
                            // Ideally extend RuntimeMessage. For now, send formatted string?
                            // Or better: Add RuntimeMessage::Animate 
                            // But RuntimeMessage definition is in lib.rs?
                            // Let's assume we can add it or repurpose String with visual cue.
                            // Actually, let's add Animate variant to RuntimeMessage in lib.rs later.
                            // For this step, I'll send a formatted string that page.tsx can parse.
                            // "[Animation] {json}"
                            let json = format!(r#"{{ "target": "{}", "property": "{}", "value": "{}", "duration": {} }}"#, 
                                target, property, target_val, duration_secs);
                            let _ = tx.send(RuntimeMessage::String(format!("[Animation] {}", json)));
                        }
                    }
                }
                Statement::EventHandler { target, event, body } => {
                    // Resolve "self" or target
                    let stack = self.context_stack.lock().unwrap();
                    let target_id = if target == "MainButton" {
                        // Hack for demo: explicit target
                        "MainButton".to_string()
                    } else if target == "self" || target == "マウス" { 
                        // If "mouse", target is likely inferred? 
                        // In draft: "マウス が 上 に あるとき" where?
                        // "カード の 上 に" or implicit context?
                        // Assuming context stack has target.
                        stack.last().cloned().unwrap_or("Screen".to_string())
                    } else {
                        target.clone()
                    };
                    drop(stack);

                    // 1. Register handler in Interpreter
                    {
                        let mut handlers = self.event_handlers.lock().unwrap();
                        handlers.insert((target_id.clone(), event.clone()), body.clone());
                    }

                    // 2. Send RegisterEvent to Frontend
                    // Frontend needs to attach listener to HTML element `target_id`.
                    // And extract animations for immediate feedback if purely visual?
                    // Actually, if we use `handle_event` re-entry, we don't need to extract animations here.
                    // The Backend will execute `AnimateStatement`.
                    // But for "hover" effects (CSS), Frontend handling is smoother?
                    // Draft says: "0.3秒 かけて 影 を 深くする".
                    // If Backend handles it, roundtrip latency might be visible?
                    // For "Clicker", click -> score update -> render is fine.
                    // For "Hover", CSS is better.
                    // But syntax is general.
                    // Let's stick to Backend execution for logic correctness (Score update).
                    // For pure animations, we might want optimization, but let's do Backend first.
                    
                    if let Ok(guard) = SCREEN_CHANNEL.lock() {
                        if let Some(tx) = &*guard {
                            // We don't send animations here anymore, just the registration order.
                            let _ = tx.send(RuntimeMessage::RegisterEvent(target_id.clone(), event.clone(), Vec::new()));
                        }
                    }
                    
                    // Log for Web Frontend Interception
                    log::info!("[RegisterEvent] {} {}", target_id, event);
                }
                Statement::AnimateStatement { duration, target, property, value } => {
                    let duration_val = self.eval_expr(duration).await;
                    let target_val_f32 = match self.eval_expr(value).await {
                         Value::Number(n) => n,
                         Value::String(s) if s == "deepen" => 20.0,
                         Value::String(s) if s == "blue" => 1.0, 
                         _ => 0.0, // Should handle colors etc properly
                    };
                    
                    let duration_secs = match duration_val {
                        Value::Number(n) => n,
                        _ => 0.0,
                    };

                    // For now, construct formatted string for frontend
                    log::info!("[Animation] Target: {}, Prop: {}, Value: {}, Duration: {}s", target, property, target_val_f32, duration_secs);

                     if let Ok(guard) = SCREEN_CHANNEL.lock() {
                        if let Some(tx) = &*guard {
                            let json = format!(r#"{{ "target": "{}", "property": "{}", "value": "{}", "duration": {} }}"#, 
                                target, property, target_val_f32, duration_secs);
                            let _ = tx.send(RuntimeMessage::String(format!("[Animation] {}", json)));
                        }
                    }
                }

                // === Eeyo: 空間・通信ステートメント (Phase 13) ===
                Statement::SpatialSearch { result, max_distance, filters } => {
                    // 距離を数値として取得
                    let distance = match self.eval_expr(max_distance).await {
                        Value::Number(n) => n,
                        Value::String(s) => {
                            // "10m" -> 10.0
                            s.trim_end_matches(char::is_alphabetic)
                                .parse::<f64>()
                                .unwrap_or(10.0)
                        }
                        _ => 10.0,
                    };
                    
                    // フィルタを変換
                    let filter_vec: Vec<(String, String)> = filters.iter()
                        .map(|f| (f.field.clone(), format!("{:?}", f.condition)))
                        .collect();
                    
                    log::info!("[Eeyo] 空間検索実行: distance={}m, filters={}", distance, filter_vec.len());
                    
                    // P2P APIを呼び出し
                    let peers = agn_spatial_search(distance, &filter_vec).await;
                    
                    // 結果を文字列として保存
                    let result_str = if peers.is_empty() {
                        "[]".to_string()
                    } else {
                        format!("[{} peers found]", peers.len())
                    };
                    
                    let mut table = self.symbol_table.lock().unwrap();
                    table.register(result, Value::String(result_str));
                }
                Statement::BeaconBroadcast { beacon_type, duration, payload: _ } => {
                    // 発信時間を取得
                    let duration_sec = if let Some(dur_expr) = duration {
                        match self.eval_expr(dur_expr).await {
                            Value::Number(n) => Some(n as u64),
                            _ => None,
                        }
                    } else {
                        None
                    };
                    
                    log::info!("[Eeyo] ビーコン発信: type={}, duration={:?}s", beacon_type, duration_sec);
                    
                    // P2P APIを呼び出し
                    if let Err(e) = agn_broadcast_beacon(beacon_type, duration_sec).await {
                        log::error!("[Eeyo] ビーコン発信失敗: {}", e);
                    }
                }
                Statement::Notify { target, message } => {
                    let target_val = self.eval_expr(target).await;
                    let message_val = self.eval_expr(message).await;
                    
                    let peer_id = match target_val {
                        Value::String(s) => s,
                        v => format!("{}", v),
                    };
                    let msg = match message_val {
                        Value::String(s) => s,
                        v => format!("{}", v),
                    };
                    
                    log::info!("[Eeyo] 通知送信: peer={}, message={}", peer_id, msg);
                    
                    // P2P APIを呼び出し
                    if let Err(e) = agn_notify_peer(&peer_id, &msg).await {
                        log::error!("[Eeyo] 通知失敗: {}", e);
                    }
                }
                Statement::TokuAccrue { target, amount } => {
                    let target_val = self.eval_expr(target).await;
                    let amount_val = self.eval_expr(amount).await;
                    
                    let user_id = match target_val {
                        Value::String(s) => s,
                        v => format!("{}", v),
                    };
                    let toku_amount = match amount_val {
                        Value::Number(n) => n as u32,
                        Value::String(s) => s.parse::<u32>().unwrap_or(10),
                        _ => 10,
                    };
                    
                    log::info!("[Eeyo] 徳加算: user={}, amount={}", user_id, toku_amount);
                    
                    // TokuManager APIを呼び出し
                    crate::p2p::agn_add_toku(&user_id, toku_amount);
                }
            }
        }
        //     let _ = handle.await;
        // }
    }

    async fn execute_verb(&self, verb: &str, value: Value) {
        execute_verb_static(verb, value).await;
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

async fn execute_verb_static(verb: &str, value: Value) {
    match verb {
        "表示する" => {
            log::info!("[Output] {}", value);
            if let Ok(guard) = SCREEN_CHANNEL.lock() {
                if let Some(tx) = &*guard {
                    let _ = tx.send(RuntimeMessage::String(value.to_string()));
                }
            }
        }
        "要約する" | "翻訳する" => {
            // AI動詞の実行
            let input = match value {
                Value::String(s) => s,
                Value::Number(n) => n.to_string(),
                _ => String::new(),
            };
            
            let runtime = crate::ai_runtime::AiRuntime::new();
            match runtime.execute_verb(verb, &input, None).await {
                Ok(result) => log::info!("[AI] Result: {}", result),
                Err(e) => log::error!("[AI Error] {}", e),
            }
        }
        _ => {
            eprintln!("Unknown verb: {}", verb);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    #[tokio::test]
    async fn test_assignment_and_display() {
        let code = "X は 42 だ";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        let interpreter = Interpreter::new();
        interpreter.execute(&program).await;
        
        let table = interpreter.symbol_table.lock().unwrap();
        match table.lookup("X") {
            Some(Value::Number(n)) => assert_eq!(*n, 42.0),
            _ => panic!("Expected X = 42"),
        }
    }

    #[tokio::test]
    async fn test_repeat_loop() {
        let code = "let X is 0\nrepeat 5 times add 1 to X end";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        let interpreter = Interpreter::new();
        interpreter.execute(&program).await;
        
        let table = interpreter.symbol_table.lock().unwrap();
        match table.lookup("X") {
            Some(Value::Number(n)) => assert_eq!(*n, 5.0),
            _ => panic!("Expected X = 5"),
        }
    }

    #[tokio::test]
    async fn test_if_statement() {
        let code = "let X is 5\nif X equals 5 then add 10 to X end";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        let interpreter = Interpreter::new();
        interpreter.execute(&program).await;
        
        let table = interpreter.symbol_table.lock().unwrap();
        match table.lookup("X") {
            Some(Value::Number(n)) => assert_eq!(*n, 15.0),
            _ => panic!("Expected X = 15"),
        }
    }
}
