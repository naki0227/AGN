//! AGN Interpreter - インタプリタ
//! ASTを直接実行する（制御構文を含む）

use crate::parser::{Condition, Expr, Program, Statement};
use crate::symbol_table::{SymbolTable, Value};
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::sync::mpsc::Sender;

use crate::graphics::animation::Animation;
use crate::bridge::{P2PBridge, UIManager};
// unused import: SocialTokuEvent

#[derive(Debug, Clone)]
pub enum RuntimeMessage {
    String(String),
    Animate(Animation),
    RegisterEvent(String, String, Vec<Animation>),
    LoadImage(String, String),
}

pub static SCREEN_CHANNEL: StdMutex<Option<Sender<RuntimeMessage>>> = StdMutex::new(None);

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
    pub rules: Arc<StdMutex<std::collections::HashMap<String, Vec<Statement>>>>,
    pub actions: Arc<StdMutex<std::collections::HashMap<String, (Vec<String>, Vec<Statement>)>>>,
    // Phase 15: Event Listeners (Event -> Vec<Statement>)
    // Key: (EventType)
    pub event_listeners: Arc<StdMutex<std::collections::HashMap<String, Vec<(Option<String>, Option<String>, Vec<Statement>)>>>>,
    
    // Phase 18: Bridges
    pub p2p: Arc<dyn P2PBridge>,
    pub ui: Arc<dyn UIManager>,
}

 impl Interpreter {
    pub fn new() -> Self {
        // デフォルトでは何もしないか、Wasm環境ならWasmBridgeをセットする（後のステップで実装）
        // 一旦、仮のBridgeをセット
        panic!("Use Interpreter::with_bridges or similar. Modularization in progress.");
    }

    pub fn with_bridges(p2p: Arc<dyn P2PBridge>, ui: Arc<dyn UIManager>) -> Self {
        Self {
            symbol_table: Arc::new(StdMutex::new(SymbolTable::new())),
            context_stack: Arc::new(StdMutex::new(Vec::new())),
            rules: Arc::new(StdMutex::new(std::collections::HashMap::new())),
            actions: Arc::new(StdMutex::new(std::collections::HashMap::new())),
            event_listeners: Arc::new(StdMutex::new(std::collections::HashMap::new())),
            event_handlers: Arc::new(StdMutex::new(std::collections::HashMap::new())),
            p2p,
            ui,
        }
    }

    pub fn with_symbol_table(symbol_table: Arc<StdMutex<SymbolTable>>, p2p: Arc<dyn P2PBridge>, ui: Arc<dyn UIManager>) -> Self {
        Self {
            symbol_table,
            context_stack: Arc::new(StdMutex::new(Vec::new())),
            rules: Arc::new(StdMutex::new(std::collections::HashMap::new())),
            actions: Arc::new(StdMutex::new(std::collections::HashMap::new())),
            event_listeners: Arc::new(StdMutex::new(std::collections::HashMap::new())),
            event_handlers: Arc::new(StdMutex::new(std::collections::HashMap::new())),
            p2p,
            ui,
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
            // AGN 2.0: Property Access (Stub)
            // AGN 2.0: Property Access
            Expr::PropertyAccess { target, property } => {
                let target_val = Box::pin(self.eval_expr(&target)).await;
                
                // Case 1: Bond property access (bond(A, B).level)
                if let Value::Bond(rel) = &target_val {
                    match property.as_str() {
                        "level" | "レベル" | "ランク" => return Value::Number(rel.level as f64),
                        "strength" | "強さ" | "親密度" => return Value::Number(rel.strength as f64),
                        "help_count" | "助けた回数" => return Value::Number(rel.help_count as f64),
                        "last_interaction" | "最後の接触" => return Value::Number(rel.last_interaction as f64),
                        _ => {} // Fallthrough
                    }
                }

                // Case 2: ID-based property access (User.Toku, Post.Author)
                if let Value::String(id) = target_val {
                    // Try as Feed Event first
                    if let Some(event) = self.p2p.get_feed_event(&id).await {
                         match property.as_str() {
                             "Author" | "作成者" | "author" => return Value::String(event.actor_id),
                             "Timestamp" | "作成日時" | "timestamp" => return Value::Number(event.timestamp as f64),
                             "Content" | "内容" | "content" => return Value::String(event.message.clone().unwrap_or_default()),
                             _ => {} // Fallthrough
                         }
                    }

                    match property.as_str() {
                        "徳" | "Toku" | "toku" => {
                            let score = self.p2p.get_toku(&id);
                            Value::Number(score as f64)
                        }
                        "rssi" | "信号強度" | "RSSI" => {
                            // Simulated RSSI for verification
                            Value::Number(-65.0)
                        }
                        "distance" | "距離" => {
                            // Simulated Distance
                            Value::Number(5.0)
                        }
                        "duration" | "接触時間" => {
                            // Simulated Contact Duration
                            Value::Number(10.0)
                        }
                        "ランク" | "Rank" | "rank" => {
                            // Dummy logic based on score
                            let score = self.p2p.get_toku(&id);
                            let rank = if score > 1000 { "徳人" } else { "一般" };
                            Value::String(rank.to_string())
                        }
                        _ => {
                            log::warn!("Unknown property: {}", property);
                            Value::Nil
                        },
                    }
                } else {
                    log::warn!("Target not found or not ID: {:?}", target_val);
                    Value::Nil
                }
            }
            // AGN 2.0: Bond (bond(A, B))
            Expr::Bond(left, right) => {
                let left_val = Box::pin(self.eval_expr(left)).await;
                let right_val = Box::pin(self.eval_expr(right)).await;
                
                if let (Value::String(l), Value::String(r)) = (left_val, right_val) {
                    let rel = self.p2p.get_bond(&l, &r);
                    Value::Bond(rel)
                } else {
                    Value::Nil
                }
            }
            // AGN 2.0: Call
            Expr::Call { name, args } => {
                // Check if it's an AI verb
                let is_ai_verb = match name.as_str() {
                    "要約する" | "summarize" | "翻訳する" | "translate" | "想像する" | "imagine" => true,
                    _ => false,
                };

                if is_ai_verb {
                    let mut arg_vals = Vec::new();
                    for arg in args {
                        arg_vals.push(Box::pin(self.eval_expr(arg)).await);
                    }
                    let input = arg_vals.get(0).map(|v| v.to_string()).unwrap_or_default();
                    let option = arg_vals.get(1).map(|v| v.to_string());
                    
                    let runtime = crate::ai_runtime::AiRuntime::new();
                    match runtime.execute_verb(name, &input, option).await {
                        Ok(result) => Value::String(result),
                        Err(e) => {
                            log::error!("[AI Error] {}", e);
                            Value::Nil
                        }
                    }
                } else if name == "get_bond" || name == "絆を取得する" {
                    let from = args.get(0).map(|a| Box::pin(self.eval_expr(a)));
                    let to = args.get(1).map(|a| Box::pin(self.eval_expr(a)));
                    
                    if let (Some(f_fut), Some(t_fut)) = (from, to) {
                        let f_val = f_fut.await;
                        let t_val = t_fut.await;
                        if let (Value::String(f), Value::String(t)) = (f_val, t_val) {
                             let rel = self.p2p.get_bond(&f, &t);
                             Value::Bond(rel)
                        } else { Value::Nil }
                    } else { Value::Nil }
                } else if name == "set_status" || name == "ステータスを設定する" {
                    let from = args.get(0).map(|a| Box::pin(self.eval_expr(a)));
                    let to = args.get(1).map(|a| Box::pin(self.eval_expr(a)));
                    let status = args.get(2).map(|a| Box::pin(self.eval_expr(a)));
                    
                    if let (Some(f_fut), Some(t_fut), Some(s_fut)) = (from, to, status) {
                        let (f, t, s) = (f_fut.await, t_fut.await, s_fut.await);
                        // TODO: Update P2P layer with status
                        // For now just log
                        log::info!("[Bond] Set status {} <-> {}: {}", f, t, s);
                        // self.p2p.set_bond_status(&f, &t, &s.to_string());
                    }
                    Value::Nil
                } else {
                    Box::pin(self.execute_action(name, args)).await
                }
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
                    (Value::Number(a), Value::Number(b)) => {
                        log::info!("Condition: {} > {} = {}", a, b, a > b);
                        a > b
                    },
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
            // AGN 2.0: Relationship Condition (Stub)
            // AGN 2.0: Relationship Condition
            Condition::HasBond(left, right) => {
                let left_val = Box::pin(self.eval_expr(left)).await;
                let right_val = Box::pin(self.eval_expr(right)).await;
                
                if let (Value::String(l), Value::String(r)) = (left_val, right_val) {
                    self.p2p.has_bond(&l, &r)
                } else {
                    false
                }
            }
            Condition::Truthy(expr) => {
                let val = self.eval_expr(expr).await;
                match val {
                    Value::Number(n) => n != 0.0,
                    Value::String(s) => !s.is_empty(),
                    Value::Bond(rel) => rel.has_bond(),
                    Value::Component { .. } => true,
                    Value::Image(_) => true,
                    Value::Nil => false,
                }
            }
        }
    }

    pub async fn execute(&self, program: &Program) {
        self.execute_statements(&program.statements).await;
    }

    async fn resolve_target_id(&self, expr: &Expr) -> String {
        match expr {
            Expr::Variable(name) => name.clone(),
            Expr::PropertyAccess { .. } | Expr::Bond(_, _) => {
                let val = self.eval_expr(expr).await;
                match val {
                    Value::String(s) => s,
                    _ => format!("{}", val),
                }
            }
            _ => format!("{:?}", expr),
        }
    }

    pub async fn execute_statements(&self, statements: &[Statement]) {
        // let mut handles = Vec::new();

        for stmt in statements {
            match stmt {
                Statement::Assignment { target, value } => {
                    let val = self.eval_expr(value).await;
                    
                    match target {
                        Expr::Variable(name) => {
                            let mut table = self.symbol_table.lock().unwrap();
                            table.register(name, val.clone());
                            
                            // Check context stack to see if we are inside a block (Implicit Child Addition)
                            // Only for UI-related values (String, Component, Image)
                            if matches!(val, Value::String(_) | Value::Component { .. } | Value::Image(_)) {
                                 drop(table); // unlock value table before locking stack
                                 
                                 let stack = self.context_stack.lock().unwrap();
                                 if let Some(parent_name) = stack.last() {
                                     let mut table = self.symbol_table.lock().unwrap();
                                     if let Some(Value::Component { children: _, .. }) = table.lookup(parent_name).cloned().as_mut() {
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
                        Expr::PropertyAccess { target: sub_target, property } => {
                            let target_val = Box::pin(self.eval_expr(sub_target)).await;
                            
                            // 特殊プロパティ更新: 徳 (User.徳 = 100)
                            if let Value::String(id) = target_val {
                                if property == "徳" || property == "Toku" {
                                    if let Value::Number(delta) = self.eval_expr(value).await {
                                        // Update Toku Score
                                        let id = self.resolve_target_id(sub_target).await;
                                        let _current = self.p2p.get_toku(&id);
                                        self.p2p.add_toku(&id, delta.max(0.0) as u32); // Simple set-via-add for now
                                    }
                                }
                            }
                        }
                        _ => {
                            log::warn!("Assignment to unsupported target type: {:?}", target);
                        }
                    }
                }
                
                Statement::ComponentDefine { target, style, component } => {
                     if let Expr::Variable(name) = target {
                         let mut table = self.symbol_table.lock().unwrap();

                         let comp_val = Value::Component { 
                             style: style.clone(), 
                             ty: component.clone(), 
                             label: Some(name.clone()),
                             children: Vec::new(),
                             layout: None,
                         };
                         
                         table.register(&name, comp_val.clone());
                         
                         // Check context stack to see if we are inside a block
                         drop(table); // unlock value table before locking stack
                         
                         let stack = self.context_stack.lock().unwrap();
                         if let Some(parent_name) = stack.last() {
                             let mut table = self.symbol_table.lock().unwrap();
                                  if let Some(Value::Component { children: _, .. }) = table.lookup(parent_name).cloned().as_mut() {
                                       let parent_val = table.get_value(parent_name);
                                       if let Value::Component { style, ty, label, mut children, layout } = parent_val {
                                           children.push(comp_val);
                                           table.update(parent_name, Value::Component { style, ty, label, children, layout });
                                       }
                                  }
                         }
                     } else {
                         log::warn!("ComponentDefine target must be a variable: {:?}", target);
                     }
                }
                Statement::Block { target, body } => {
                    let target_id = self.resolve_target_id(target).await;
                    // Push target to stack
                    {
                        let mut stack = self.context_stack.lock().unwrap();
                        stack.push(target_id.clone());
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
                    let target_id = self.resolve_target_id(target).await;
                    // Set layout on target. 
                    // If target is "これら", use current context (parent).
                    let target_name = if target_id == "これら" {
                        let stack = self.context_stack.lock().unwrap();
                        if let Some(parent) = stack.last() {
                            parent.clone()
                        } else {
                            eprintln!("Error: 'これら' used outside of block");
                            continue;
                        }
                    } else {
                        target_id
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
                     if let Expr::Variable(name) = target {
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
                         self.ui.send_runtime_message(RuntimeMessage::LoadImage(name.clone(), path_str.clone()));
                         // Keep Value::Image for SymbolTable if needed
                         let val = Value::Image(path_str);
                         
                         let mut table = self.symbol_table.lock().unwrap();
                         table.register(&name, val.clone());
                         
                         drop(table);
                     }
                }
                Statement::BinaryOp { target, operand, verb } => {
                    let op_val = self.eval_expr(operand).await;
                    
                    match target {
                        Expr::Variable(name) => {
                            // Handle "Screen" Display (e.g. "MainButton を 画面 に 表示する")
                            if (name == "Screen" || name == "Screen.Center") && verb == "表示する" {
                                 log::info!("[Output] {}", op_val);
                                 self.ui.send_runtime_message(RuntimeMessage::String(op_val.to_string()));
                                 continue;
                            }

                            let mut table = self.symbol_table.lock().unwrap();
                            // Numeric Operations
                            if let Some(Value::Number(current)) = table.lookup(name).cloned() {
                                if let Value::Number(op_num) = op_val {
                                     let result = match verb.as_str() {
                                        "足す" | "加算する" | "増やす" => current + op_num,
                                        "引く" | "減らす" => current - op_num,
                                        "掛ける" => current * op_num,
                                        "割る" => if op_num != 0.0 { current / op_num } else { current },
                                        _ => current,
                                     };
                                     table.update(name, Value::Number(result));
                                }
                            }
                            // Component Operations (e.g. "つなぐ")
                            else if let Some(Value::Component { .. }) = table.lookup(name).cloned() {
                                if verb == "つなぐ" {
                                    let parent_val = table.get_value(name);
                                    if let Value::Component { style, ty, label, mut children, layout } = parent_val {
                                        children.push(op_val.clone());
                                        table.update(name, Value::Component { style, ty, label, children, layout });
                                    }
                                }
                            }
                        }
                        Expr::PropertyAccess { target: sub_target, property } => {
                            let target_val = Box::pin(self.eval_expr(sub_target)).await;
                            
                            // 徳の更新
                            if let Value::String(id) = target_val {
                                if property == "徳" || property == "Toku" {
                                    if let Value::Number(n) = op_val {
                                        match verb.as_str() {
                                            "足す" | "加算する" | "増やす" => self.p2p.add_toku(&id, n as u32),
                                            "引く" | "減らす" => self.p2p.subtract_toku(&id, n as u32),
                                            _ => {}
                                        }
                                    }
                                }
                            }
                            // 絆の更新 via プロパティ (bond(A, B).強さ を 増やす)
                            else if let Value::Bond(_rel) = target_val {
                                // Need IDs to update bond. Relationship struct doesn't have IDs.
                                // We might need to handle this in Expr::Bond directly or fetch IDs if possible.
                                // Let's assume binary op on Bond sub-expr handles it.
                                log::warn!("Direct property update on anonymous Bond not yet supported. Use bond(A, B) target.");
                            }
                        }
                        Expr::Bond(left, right) => {
                            let left_val = Box::pin(self.eval_expr(left)).await;
                            let right_val = Box::pin(self.eval_expr(right)).await;
                            
                            if let (Value::String(l), Value::String(r)) = (left_val, right_val) {
                                let num_val = if let Value::Number(n) = op_val { Some(n) } else { None };
                                if verb == "深くする" || verb == "増やす" || verb == "deepen" || verb == "increase" {
                                    if let Some(n) = num_val {
                                        self.p2p.deepen_bond(&l, &r, n as u32);
                                    } else {
                                        self.p2p.deepen_bond(&l, &r, 1); // default 1
                                    }
                                }
                            }
                        }
                        _ => {
                            log::warn!("BinaryOp on unsupported target type: {:?}", target);
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
                    let interpreter_clone = self.clone();
                    
                    let _handle = spawn_async(async move {
                        interpreter_clone.execute_verb(&verb, val).await;
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
                    let result_id = self.resolve_target_id(result).await;

                    match runtime.execute_verb(verb, &input_str, option_val).await {
                        Ok(ai_result) => {
                            log::info!("[AI] {} result: {}", verb, &ai_result);
                            let mut table = self.symbol_table.lock().unwrap();
                            table.register(&result_id, Value::String(ai_result));
                        }
                        Err(e) => {
                            log::error!("[AI Error] {}: {}", verb, e);
                            let mut table = self.symbol_table.lock().unwrap();
                            table.register(&result_id, Value::String(format!("[AI Error: {}]", e)));
                        }
                    }
                }
                Statement::ScreenOp { operand } => {
                    let val = self.eval_expr(operand).await;
                    log::info!("[Output] {}", val);
                    if let Value::String(s) = val {
                         self.ui.send_runtime_message(RuntimeMessage::String(s));
                    }
                }
                Statement::DelayStatement { duration, body } => {
                    let duration_val = self.eval_expr(duration).await;
                    if let Value::Number(secs) = duration_val {
                        // Async delay
                        let sleep_ms = (secs * 1000.0) as u64;
                        crate::utils::sleep(sleep_ms).await;
                        
                        Box::pin(self.execute_statements(body)).await;
                    }
                }
                
                Statement::EventHandler { target, event, body } => {
                    // Resolve target ID
                    let target_id = self.resolve_target_id(target).await;
                    
                    // Register handler in Interpreter
                    {
                        let mut handlers = self.event_handlers.lock().unwrap();
                        handlers.insert((target_id.clone(), event.clone()), body.clone());
                    }

                    self.ui.send_runtime_message(RuntimeMessage::RegisterEvent(target_id.clone(), event.clone(), Vec::new()));
                    
                    log::info!("[RegisterEvent] {} {}", target_id, event);
                }
                Statement::AnimateStatement { duration, target, property, value } => {
                    let duration_val = self.eval_expr(duration).await;
                    let target_id = self.resolve_target_id(target).await;
                    let target_val_f32 = match self.eval_expr(value).await {
                         Value::Number(n) => n,
                         Value::String(s) if s == "deepen" => 20.0,
                         Value::String(s) if s == "blue" => 1.0, 
                         _ => 0.0,
                    };
                    
                    let duration_secs = match duration_val {
                        Value::Number(n) => n,
                        _ => 0.0,
                    };

                    log::info!("[Animation] Target: {}, Prop: {}, Value: {}, Duration: {}s", target_id, property, target_val_f32, duration_secs);

                    let json = format!(r#"{{ "target": "{}", "property": "{}", "value": "{}", "duration": {} }}"#, 
                        target_id, property, target_val_f32, duration_secs);
                    self.ui.send_runtime_message(RuntimeMessage::String(format!("[Animation] {}", json)));
                }

                // === Eeyo: 空間・通信ステートメント (Phase 13) ===
                Statement::SpatialSearch { result, max_distance, filters } => {
                    let result_id = self.resolve_target_id(result).await;
                    // ... (rest of SpatialSearch logic)
                    let distance = match self.eval_expr(max_distance).await {
                        Value::Number(n) => n,
                        Value::String(s) => {
                            s.trim_end_matches(char::is_alphabetic)
                                .parse::<f64>()
                                .unwrap_or(10.0)
                        }
                        _ => 10.0,
                    };
                    
                    let filter_vec: Vec<(String, String)> = filters.iter()
                        .map(|f| (f.field.clone(), format!("{:?}", f.condition)))
                        .collect();
                    
                    let peers = self.p2p.spatial_search(distance, &filter_vec).await;
                    
                    let result_str = if peers.is_empty() {
                        "[]".to_string()
                    } else {
                        format!("[{} peers found]", peers.len())
                    };
                    
                    let mut table = self.symbol_table.lock().unwrap();
                    table.register(&result_id, Value::String(result_str));
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
                    self.p2p.broadcast_beacon(beacon_type, duration_sec).await;
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
                    let _ = self.p2p.notify_peer(&peer_id, &msg).await;
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
                    self.p2p.add_toku(&user_id, toku_amount);
                }

                // === AGN 2.0: Social Layer Stubs ===
                Statement::RuleDefinition { name, body } => {
                    let mut rules = self.rules.lock().unwrap();
                    rules.insert(name.clone(), body.clone());
                }
                Statement::ActionDefinition { name, params, body } => {
                     log::info!("[AGN 2.0] Action defined: {}({:?})", name, params);
                     let mut actions = self.actions.lock().unwrap();
                     actions.insert(name.clone(), (params.clone(), body.clone()));
                }
                
                // Phase 15: Event Listener Register
                Statement::EventListener { event_type, from_var, to_var, body } => {
                    let mut listeners = self.event_listeners.lock().unwrap();
                    listeners.entry(event_type.clone())
                        .or_insert_with(Vec::new)
                        .push((from_var.clone(), to_var.clone(), body.clone()));
                    log::info!("[AGN] Registered EventListener for {}", event_type);
                }

                Statement::VariableUpdate { target, value, verb } => {
                    // 1. Evaluate value to update with
                    let val = self.eval_expr(value).await;
                    let amount = match val {
                        Value::Number(n) => n as i32,
                        _ => 0,
                    };
                    
                    // 2. Resolve target
                    // Supported patterns:
                    // - User.Toku ("徳")
                    // - Variable (local var)
                    
                    match target {
                        Expr::PropertyAccess { target: obj_expr, property } => {
                            let obj_val = self.eval_expr(&obj_expr).await;
                            
                            if let Value::String(user_id) = obj_val {
                                match property.as_str() {
                                    "徳" | "Toku" | "toku" => {
                                        match verb.as_str() {
                                            "増やす" | "increase" | "add" => {
                                                self.p2p.add_toku(&user_id, amount as u32);
                                            }
                                            "減らす" | "decrease" | "subtract" => {
                                                self.p2p.subtract_toku(&user_id, amount as u32);
                                            }
                                            "更新する" | "update" | "set" => {
                                                log::warn!("徳スコアの直接設定は未サポートです。増減を使用してください。");
                                            }
                                            _ => {}
                                        }
                                    }
                                    "rssi" | "信号強度" => {
                                        // Read-only property (simulated)
                                        log::warn!("RSSI is read-only.");
                                    }
                                    "distance" | "距離" => {
                                        log::warn!("Distance is read-only.");
                                    }
                                    _ => {
                                        log::warn!("Unknown property update: .{}", property);
                                    }
                                }
                            }
                        }
                        Expr::Variable(var_name) => {
                             // Local variable update
                             let mut table = self.symbol_table.lock().unwrap();
                             // Special case for FeedList
                             if var_name == "FeedList" && (verb == "更新する" || verb == "update") {
                                 drop(table); // release lock before await
                                 self.update_feed_ui().await;
                                 return;
                             }

                             if let Value::Number(current) = table.get_value(&var_name) {
                                  match verb.as_str() {
                                      "増やす" | "increase" => {
                                          table.update(&var_name, Value::Number(current + amount as f64));
                                      }
                                      "減らす" | "decrease" => {
                                          table.update(&var_name, Value::Number(current - amount as f64));
                                      }
                                      "更新する" | "update" | "set" => {
                                          if let Value::Number(new_val) = val {
                                              table.update(&var_name, Value::Number(new_val));
                                          }
                                      }
                                      _ => {}
                                  }
                             }
                        }
                        Expr::Bond(left, right) => {
                             let from_val = self.eval_expr(left).await;
                             let to_val = self.eval_expr(right).await;
                             if let (Value::String(from), Value::String(to)) = (from_val, to_val) {
                                 if verb == "深くする" || verb == "deepen" {
                                     self.p2p.deepen_bond(&from, &to, amount as u32);
                                 }
                             }
                        }
                        _ => {
                            // Property access via Variable (User.Toku) is handled above if parser structured it as PropertyAccess.
                            // If parser didn't, we can't easily handle it here without re-parsing/eval structure.
                        }
                    }
                }
                Statement::ReturnStatement { value } => {
                    let val = self.eval_expr(value).await;
                    let mut table = self.symbol_table.lock().unwrap();
                    table.register("結果", val);
                }
                Statement::ActionCall { name, args } => {
                    Box::pin(self.execute_action(name, args)).await;
                }
            }
        }
        //     let _ = handle.await;
        // }
    }


    pub fn fork_with_table(&self, symbol_table: Arc<StdMutex<SymbolTable>>) -> Self {
        Self {
            symbol_table,
            context_stack: Arc::new(StdMutex::new(Vec::new())),
            event_handlers: self.event_handlers.clone(),
            rules: self.rules.clone(),
            actions: self.actions.clone(),
            event_listeners: self.event_listeners.clone(),
            p2p: self.p2p.clone(),
            ui: self.ui.clone(),
        }
    }

    pub async fn execute_action(&self, name: &str, args: &[Expr]) -> Value {
        let (params, body) = {
            let actions = self.actions.lock().unwrap();
            match actions.get(name) {
                Some(data) => data.clone(),
                None => {
                    log::error!("Action not found: {}", name);
                    return Value::Nil;
                }
            }
        };

        // Evaluate arguments
        let mut arg_values = Vec::new();
        for arg in args {
            arg_values.push(Box::pin(self.eval_expr(arg)).await);
        }

        // Create scoped table
        let mut table = SymbolTable::new();
        // Bind params
        for (i, param_name) in params.iter().enumerate() {
            if i < arg_values.len() {
                table.register(param_name, arg_values[i].clone());
            }
        }
        
        let scoped_interpreter = self.fork_with_table(Arc::new(StdMutex::new(table)));
        Box::pin(scoped_interpreter.execute_statements(&body)).await;
        
        let result = scoped_interpreter.symbol_table.lock().unwrap().get_value("結果");
        result
    }

    pub async fn execute_rule(&self, rule_name: &str, viewer: &str, post_id: &str) -> i32 {
        let rules_guard = self.rules.lock().unwrap();
        if let Some(body) = rules_guard.get(rule_name) {
            // Create scoped interpreter
            let mut table = SymbolTable::new();
            
            // Inject Context
            table.register("優先度", Value::Number(0.0));
            table.register("priority", Value::Number(0.0));
            table.register("Viewer", Value::String(viewer.to_string()));
            table.register("閲覧者", Value::String(viewer.to_string()));
            table.register("Post", Value::String(post_id.to_string()));
            table.register("投稿", Value::String(post_id.to_string()));
            
            let symbol_table = Arc::new(StdMutex::new(table));
            let scoped_interpreter = self.fork_with_table(symbol_table);
            
            // Execute
            // We need to release the rules lock before executing, as statements might access rules?
            // Actually execute_statements doesn't lock rules except for definition.
            // But to be safe, clone body.
            let body_clone = body.clone();
            drop(rules_guard);
            
            Box::pin(scoped_interpreter.execute_statements(&body_clone)).await;
            
            // Retrieve result
            // Retrieve result
            let table = scoped_interpreter.symbol_table.lock().unwrap();
            let val_ja = match table.get_value("優先度") {
                Value::Number(n) => n,
                _ => 0.0,
            };
            let val_en = match table.get_value("priority") {
                Value::Number(n) => n,
                _ => 0.0,
            };
            
            (val_ja + val_en) as i32
        } else {
            0
        }
    }

    pub async fn update_feed_ui(&self) {
        log::info!("[Interpreter] Updating Feed UI...");
        let events = self.p2p.get_all_feed_events().await;
        
        let mut event_scores = Vec::new();
        
        // Calculate priority for each event
        for event in &events {
            let score = self.execute_rule("KizatoFeed", "Me", &event.id).await;
            event_scores.push((event, score));
        }
        
        // Sort by priority (Descending)
        event_scores.sort_by(|a, b| b.1.cmp(&a.1));
        
        let mut children = Vec::new();
        
        // Take top 20
        for (event, score) in event_scores.into_iter().take(20) {
            let mut event_children = Vec::new();
            
            // Header: Author Name + Priority Badge
            let header = Value::Component {
                style: "Header".to_string(),
                ty: "コンテナ".to_string(),
                label: None,
                children: vec![
                    Value::String(format!("{} さん", event.actor_id)),
                    Value::Component {
                         style: "Badge".to_string(), // New style needed? 
                         ty: "ラベル".to_string(),
                         label: None,
                         children: vec![Value::String(format!("優先度: {}", score))],
                         layout: None,
                    }
                ],
                layout: Some("horizontal".to_string()),
            };
            event_children.push(header);
            
            // Content
            if let Some(msg) = &event.message {
                event_children.push(Value::String(msg.clone()));
            }
            
            // Image
            if let Some(url) = &event.image_url {
                event_children.push(Value::Image(url.clone()));
            }
            
            // Interaction (Toku Button)
            let button_id = format!("Like_{}", event.id);
            let button = Value::Component {
                style: "Button".to_string(),
                ty: "ボタン".to_string(),
                label: Some(button_id),
                children: vec![Value::String("徳を送る".to_string())],
                layout: None,
            };
            event_children.push(button);

            // Container for Post
            let post_comp = Value::Component {
                style: "PostCard".to_string(),
                ty: "コンテナ".to_string(),
                label: Some(format!("Post_{}", event.id)),
                children: event_children,
                layout: Some("vertical".to_string()),
            };
            
            children.push(post_comp);
        }
        
        let mut table = self.symbol_table.lock().unwrap();
        // Look for FeedList and update checks
        // We use lookup to check existence, but update to set new value.
        // If "FeedList" exists (even as different type or Nil), we verify or overwrite.
        // Assuming user defined `FeedList` as component in AGN.
        
        // Check if FeedList exists
        if table.lookup("FeedList").is_some() {
             // Retrieve checking type? 
             // We just overwrite content but keep style/ty if possible?
             // Since we construct children, we can just update children if we get the old value.
             let old_val = table.get_value("FeedList");
             if let Value::Component { style, ty, label, .. } = old_val {
                 table.update("FeedList", Value::Component {
                     style,
                     ty,
                     label,
                     children,
                     layout: Some("vertical".to_string()),
                 });
             }
        }
    }

    pub async fn handle_ui_event(&self, event_id: &str) {
        log::info!("[Interpreter] UI Event: {}", event_id);
        
        // Handle Toku Like Button
        if event_id.starts_with("Like_") {
            let post_id = &event_id[5..]; // "Like_".len() == 5
            log::info!("[Interpreter] Toku Like clicked for post: {}", post_id);
            
            // Extract Actor ID from Post ID if possible?
            // We need to look up the event from P2PManager to find the target (Author).
            if let Some(event) = self.p2p.get_feed_event(post_id).await {
                let author = event.actor_id;
                log::info!("[Interpreter] Sending Toku to Author: {}", author);
                
                // Add Toku (1 point)
                self.p2p.add_toku(&author, 1);
                
                // Notify User
                self.ui.notify(&format!("徳を送りました！ (to {})", author));
            } else {
                 log::warn!("[Interpreter] Unhandled UI event: {}", event_id);
            }
        }
    }

    pub async fn trigger_event(&self, event_type: &str, from_id: &str, to_id: &str) {
        log::info!("[AGN] Triggering event: {} ({} -> {})", event_type, from_id, to_id);
        
        let handlers = {
            let map = self.event_listeners.lock().unwrap();
            map.get(event_type).cloned()
        };
        
        if let Some(handler_list) = handlers {
            for (from_var, to_var, body) in handler_list {
                {
                    let mut table = self.symbol_table.lock().unwrap();
                    if let Some(f) = &from_var {
                        table.register(f, Value::String(from_id.to_string()));
                    }
                    if let Some(t) = &to_var {
                        table.register(t, Value::String(to_id.to_string()));
                    }
                }
                self.execute_statements(&body).await;
            }
        }
    }

    pub async fn execute_verb(&self, verb: &str, value: Value) {
        let _input = value.to_string();
        let _option: Option<String> = None;

        match verb {
            "get_bond" | "絆を取得する" => {
                // Usually returns a value, but here it's a verb (Statement).
                // Logic already in eval_expr/execute_statements
            }
            "display" | "表示する" | "出す" => {
                self.ui.notify(&value.to_string());
            }
            "broadcast" | "発信する" => {
                self.p2p.broadcast_beacon(&value.to_string(), None).await;
            }
            _ => {
                log::warn!("Unhandled verb: {}", verb);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::bridge::mock::MockP2PBridge;

    #[tokio::test]
    async fn test_assignment_and_display() {
        let code = "X は 42 だ";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        let p2p = Arc::new(crate::bridge::mock::MockP2PBridge::new());
        let ui = Arc::new(crate::bridge::mock::MockUIManager);
        let interpreter = Interpreter::with_bridges(p2p, ui);
        interpreter.execute(&program).await;
        
        let table = interpreter.symbol_table.lock().unwrap();
        match table.lookup("X") {
            Some(Value::Number(n)) => assert_eq!(*n, 42.0),
            _ => panic!("Expected X = 42"),
        }
    }

    #[tokio::test]
    async fn test_repeat_loop() {
        let code = "X は 0 だ\nrepeat 5 times increase X by 1 end";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        let p2p = Arc::new(crate::bridge::mock::MockP2PBridge::new());
        let ui = Arc::new(crate::bridge::mock::MockUIManager);
        let interpreter = Interpreter::with_bridges(p2p, ui);
        interpreter.execute(&program).await;
        
        let table = interpreter.symbol_table.lock().unwrap();
        match table.lookup("X") {
            Some(Value::Number(n)) => assert_eq!(*n, 5.0),
            _ => panic!("Expected X = 5, got {:?}", table.lookup("X")),
        }
    }

    #[tokio::test]
    async fn test_if_statement() {
        let code = "X は 5 だ\nif X equals 5 then increase X by 10 end";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        let p2p = Arc::new(crate::bridge::mock::MockP2PBridge::new());
        let ui = Arc::new(crate::bridge::mock::MockUIManager);
        let interpreter = Interpreter::with_bridges(p2p, ui);
        interpreter.execute(&program).await;
        
        let table = interpreter.symbol_table.lock().unwrap();
        match table.lookup("X") {
            Some(Value::Number(n)) => assert_eq!(*n, 15.0),
            _ => panic!("Expected X = 15, got {:?}", table.lookup("X")),
        }
    }

    #[tokio::test]
    async fn test_feed_priority_rule() {
        let _ = env_logger::builder().is_test(true).try_init();
        
        let p2p = Arc::new(crate::bridge::mock::MockP2PBridge::new());
        let ui = Arc::new(crate::bridge::mock::MockUIManager);
        let interpreter = Interpreter::with_bridges(p2p.clone(), ui);

        // Setup User Score
        let user_id = "Influencer";
        p2p.add_toku(user_id, 900); // Initial 100 + 900 = 1000

        // Define Rule
        let rule_code = r#"
            rule HighTokuPriority
                if Post.Author.徳 > 500 then
                    increase priority by 50
                end
            end
        "#;
        // Parse Rule
        let mut lexer = Lexer::new(rule_code);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        interpreter.execute(&program).await;

        // Setup Post
        let event = crate::p2p::SocialTokuEvent::new(user_id, "Target", crate::p2p::SocialEventType::TokuSent { amount: 10 });
        let post_id = event.id.clone();
        p2p.inject_feed_event(event);

        // Execute Rule
        let priority = interpreter.execute_rule("HighTokuPriority", "Viewer", &post_id).await;
        
        assert_eq!(priority, 50);
    }

    #[tokio::test]
    async fn test_social_simulator_action() {
        let _ = env_logger::builder().is_test(true).try_init();
        
        let p2p = Arc::new(crate::bridge::mock::MockP2PBridge::new());
        let ui = Arc::new(crate::bridge::mock::MockUIManager);
        let interpreter = Interpreter::with_bridges(p2p.clone(), ui);

        // Define Action logic
        // "徳" is a keyword, so use a name that doesn't start with it if possible, 
        // or ensure lexer matches the whole noun. Here we use "奉納".
        let code = r#"
            action 奉納(送り手, 受け手, 金額)
                送り手.徳 に 金額 を 減らす
                受け手.徳 に 金額 を 増やす
                bond(送り手, 受け手) を 深くする
            end

            奉納("Alice", "Bob", 50)
        "#;

        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        // Setup initial toku
        p2p.add_toku("Alice", 100);
        p2p.add_toku("Bob", 100);

        interpreter.execute(&program).await;
        
        // Alice: 200 - 50 = 150
        // Bob: 200 + 50 = 250
        assert_eq!(p2p.get_toku("Alice"), 150);
        assert_eq!(p2p.get_toku("Bob"), 250);
        
        let bond = p2p.get_bond("Alice", "Bob");
        assert!(bond.strength > 0);
    }

    #[tokio::test]
    async fn test_event_listener_execution() {
        let _ = env_logger::builder().is_test(true).try_init();
        
        let p2p = Arc::new(crate::bridge::mock::MockP2PBridge::new());
        let ui = Arc::new(crate::bridge::mock::MockUIManager);
        let interpreter = Interpreter::with_bridges(p2p.clone(), ui);

        // on Event(HelpGiven) from Alice to Bob
        let code = r#"
            on Event(HelpGiven) from Alice to Bob {
                Alice.徳 に 50 を 増やす
                bond(Alice, Bob) を 深くする
            }
        "#;
        
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        // Register listeners
        interpreter.execute(&program).await;
        
        // Setup initial state
        p2p.add_toku("Alice", 100); // 100 + 100 = 200
        
        // Trigger event
        interpreter.trigger_event("HelpGiven", "Alice", "Bob").await;
        
        // Check results
        // Alice: 200 + 50 = 250
        let toku = p2p.get_toku("Alice");
        assert_eq!(toku, 250);
        
        // Bond
        let bond = p2p.get_bond("Alice", "Bob");
        assert!(bond.strength > 0);
    }

    #[tokio::test]
    async fn test_umeda_verification() {
        let _ = env_logger::builder().is_test(true).try_init();
        
        let p2p = Arc::new(crate::bridge::mock::MockP2PBridge::new());
        let ui = Arc::new(crate::bridge::mock::MockUIManager);
        let interpreter = Interpreter::with_bridges(p2p.clone(), ui);

        // Define Logic: RSSI > -70 && Duration > 5 -> Deepen Bond
        let code = r#"
            on Event(ProximityDetected) from P to Me {
                if P.rssi > -70.0 then
                    if P.duration > 5.0 then
                        bond(Me, P) を 深くする
                    end
                end
            }
        "#;
        
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        // Register listeners
        interpreter.execute(&program).await;
        
        // Setup initial bond
        p2p.deepen_bond("Me", "Stranger", 0); // Init (creates bond with strength 10)
        let rel_initial = p2p.get_bond("Me", "Stranger");
        assert_eq!(rel_initial.strength, 10, "Initial bond strength should be 10");
        
        // Trigger Event
        interpreter.trigger_event("ProximityDetected", "Stranger", "Me").await;
        
        // Verify Bond Deepened
        // Note: Default deepen amount is 1. 10 + 1 = 11.
        let rel = p2p.get_bond("Me", "Stranger");
        assert_eq!(rel.strength, 11, "Bond strength should increase by 1");
    }
}
