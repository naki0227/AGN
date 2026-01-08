//! AGN Interpreter - インタプリタ
//! ASTを直接実行する（制御構文を含む）

use crate::parser::{Condition, Expr, Program, Statement};
use crate::symbol_table::{SymbolTable, Value};
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::sync::mpsc::Sender;

use crate::graphics::animation::Animation;

#[derive(Debug, Clone)]
pub enum RuntimeMessage {
    String(String),
    Animate(Animation),
    RegisterEvent(String, String, Vec<Animation>),
    LoadImage(String, String),
}

pub static SCREEN_CHANNEL: StdMutex<Option<Sender<RuntimeMessage>>> = StdMutex::new(None);
use tokio::sync::Mutex;

pub struct Interpreter {
    symbol_table: Arc<StdMutex<SymbolTable>>,
    context_stack: Arc<StdMutex<Vec<String>>>,
}

 impl Interpreter {
    pub fn new() -> Self {
        Self {
            symbol_table: Arc::new(StdMutex::new(SymbolTable::new())),
            context_stack: Arc::new(StdMutex::new(Vec::new())),
        }
    }

    pub fn with_symbol_table(symbol_table: Arc<StdMutex<SymbolTable>>) -> Self {
        Self {
            symbol_table,
            context_stack: Arc::new(StdMutex::new(Vec::new())),
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
        }
    }

    pub async fn execute(&self, program: &Program) {
        self.execute_statements(&program.statements).await;
    }

    async fn execute_statements(&self, statements: &[Statement]) {
        let mut handles = Vec::new();

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
                    let mut table = self.symbol_table.lock().unwrap();
                    
                    if let Some(Value::Number(current)) = table.lookup(target).cloned() {
                        if let Value::Number(op_num) = op_val {
                             // ...
                             // Simplified just to match lock() call
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
                }
                Statement::UnaryOp { operand, verb } => {
                    let val = self.eval_expr(operand).await;
                    self.execute_verb(verb, val).await;
                }
                Statement::AsyncOp { operand, verb } => {
                    let val = self.eval_expr(operand).await;
                    let verb = verb.clone();
                    
                    let handle = tokio::spawn(async move {
                        execute_verb_static(&verb, val).await;
                    });
                    handles.push(handle);
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
                Statement::AiOp { result, input, verb, options: _ } => {
                    let input_val = self.eval_expr(input).await;
                    let input_str = match input_val {
                        Value::String(s) => s,
                        Value::Number(n) => n.to_string(),
                        _ => String::new(),
                    };
                    
                    let runtime = crate::ai_runtime::AiRuntime::new();
                    match runtime.execute_verb(verb, &input_str).await {
                        Ok(ai_result) => {
                            let mut table = self.symbol_table.lock().unwrap();
                            table.register(result, Value::String(ai_result));
                        }
                        Err(e) => eprintln!("AI Error: {}", e),
                    }
                }
                Statement::ScreenOp { operand } => {
                    let val = self.eval_expr(operand).await;
                    println!("[Screen] {}", val);
                    
                    if let Ok(guard) = SCREEN_CHANNEL.lock() {
                        if let Some(tx) = &*guard {
                            let _ = tx.send(RuntimeMessage::String(val.to_string()));
                        }
                    }
                }
                Statement::EventHandler { target, event, body } => {
                    // event is passed from parser ("hover", "click", "drag", etc.)
                    
                    // Resolve "self" to current context target
                    let stack = self.context_stack.lock().unwrap();
                    // If target is "self", use stack top. Else use target name?
                    let target_id = if target == "self" {
                        stack.last().cloned().unwrap_or("Screen".to_string())
                    } else {
                        target.clone()
                    };
                    drop(stack); // unlock

                    // Extract animations from body
                    let mut animations = Vec::new();
                    for stmt in body {
                        if let Statement::Animate { duration, property, target_value } = stmt {
                             // Resolve value
                            let target_val_f32 = match target_value {
                                Expr::Number(n) => *n as f32,
                                Expr::String(s) if s == "deepen" => 20.0,
                                Expr::String(s) if s == "水色" => 1.0, 
                                _ => 0.0,
                            };
                            
                            animations.push(Animation {
                                target_id: target_id.clone(),
                                property: property.clone(),
                                start_value: 0.0, // Managed by controller
                                end_value: target_val_f32,
                                start_time: std::time::Instant::now(), // Placeholder
                                duration: std::time::Duration::from_secs_f64(*duration),
                                easing: crate::graphics::animation::Easing::EaseInOut,
                            });
                        }
                    }

                    if let Ok(guard) = SCREEN_CHANNEL.lock() {
                        if let Some(tx) = &*guard {
                            let _ = tx.send(RuntimeMessage::RegisterEvent(target_id, event.clone(), animations));
                        }
                    }
                }
                Statement::Animate { duration, property, target_value } => {
                    // Check if we are in a block? Or just animate the "current target"?
                    // The syntax is "[Time] かけて [Property] を [Value] にする"
                    // It doesn't explicitly state the target object if outside a block.
                    // But usually it's used inside a block or refers to "This/Self".
                    // Or maybe we should track the "last referenced object"?
                    // For now, let's assume it applies to the "Root" or "Card" if not specific?
                    // "カード は ...。 0.3秒かけて..." -> implicit subject?
                    // Actually, the parser for current demo creates syntax:
                    // "マウス が 上 に あるとき" (Event Handler)
                    //    "0.3秒 かけて 影 を 深くする"
                    // Here, target is implicit (the object having the event handler).
                    // In `execute_statements`, we don't have implicit 'self' reference passed down easily.
                    // We might need to store `self_id` in context stack or pass it.
                    
                    // Quick fix: Use context stack. If inside a block/handler, top of stack is target.
                    let stack = self.context_stack.lock().unwrap();
                    let target_id = stack.last().cloned().unwrap_or("Screen".to_string());
                    
                    let target_val_f32 = match target_value {
                        Expr::Number(n) => *n as f32,
                        Expr::String(s) if s == "deepen" => 20.0, // Hardcoded logic for "deepen"
                        Expr::String(s) if s == "水色" => 1.0, // Hack: Color mapping is complex. Float for now.
                        Expr::Variable(_) => 0.0, // Resolve var?
                        _ => 0.0,
                    };
                    
                    let anim = Animation {
                        target_id,
                        property: property.clone(),
                        start_value: 0.0, // Needs current value from State? State has it. Controller handles it?
                                          // Controller needs start value. If passed 0, it jumps.
                                          // Better to let Controller read current value if start_value is None (or separate flag).
                                          // For now, pass 0.0 and handle in controller?
                                          // Let's modify Animation struct to have optional start?
                        end_value: target_val_f32,
                        start_time: std::time::Instant::now(), // Will be reset by receiver likely
                        duration: std::time::Duration::from_secs_f64(*duration),
                        easing: crate::graphics::animation::Easing::EaseInOut,
                    };
                    
                    if let Ok(guard) = SCREEN_CHANNEL.lock() {
                        if let Some(tx) = &*guard {
                            let _ = tx.send(RuntimeMessage::Animate(anim));
                        }
                    }
                }
            }
        }

        for handle in handles {
            let _ = handle.await;
        }
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
            println!("{}", value);
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
            match runtime.execute_verb(verb, &input).await {
                Ok(result) => println!("{}", result),
                Err(e) => eprintln!("AI Error: {}", e),
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
