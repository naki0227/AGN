
pub mod lexer;
pub mod parser;
pub mod symbol_table;
pub mod interpreter;
pub mod normalizer;
pub mod type_inferencer;
pub mod ai_analyzer;
pub mod codegen;
pub mod compiler;
pub mod memory;
pub mod ai_runtime;
pub mod web_generator;
#[cfg(not(target_arch = "wasm32"))]
pub mod native_window;
pub mod graphics;
pub mod utils;
// Eeyo: P2P通信層
pub mod p2p;
pub mod bridge;


#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use winit::platform::web::WindowExtWebSys;

#[cfg(target_arch = "wasm32")]
use std::sync::Mutex as StdMutex;

#[cfg(target_arch = "wasm32")]
use lazy_static::lazy_static;

#[cfg(target_arch = "wasm32")]
lazy_static! {
    static ref GLOBAL_INTERPRETER: StdMutex<Option<crate::interpreter::Interpreter>> = StdMutex::new(None);
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn run_script(source: String, _canvas_id: Option<String>) -> Result<(), JsValue> {
    // 1. Setup Logging
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Info).ok();

    log::info!("[AGN] Parsing source...");

    // 2. Parse
    let mut lexer = crate::lexer::Lexer::new(&source);
    let tokens = lexer.tokenize();
    
    // Debug: Log first 20 tokens
    log::info!("[AGN Debug] First 20 tokens:");
    for (i, tok) in tokens.iter().take(20).enumerate() {
        log::info!("  Token {}: {:?}", i, tok);
    }
    
    let mut parser = crate::parser::Parser::new(tokens);
    let program = match parser.parse() {
        Ok(p) => p,
        Err(e) => return Err(JsValue::from_str(&format!("Parse Error: {}", e))),
    };

    log::info!("[AGN] Parsed {} statements. Executing...", program.statements.len());

    // 3. Execute (Interpreter - Console Output Only)
    // Create bridges
    use crate::bridge::std_bridge::{StdP2PBridge, StdUIManager};
    let p2p = Arc::new(StdP2PBridge);
    let ui = Arc::new(StdUIManager);
    let interpreter = crate::interpreter::Interpreter::with_bridges(p2p, ui);
    
    // Execute main script
    interpreter.execute(&program).await;
    
    // Persist interpreter for events
    let mut guard = GLOBAL_INTERPRETER.lock().unwrap();
    *guard = Some(interpreter);

    log::info!("[AGN] Execution complete. Session active.");

    Ok(())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn handle_event(target: String, event_type: String) {
    // Attempt to lock interpreter
    // Note: Mutex in Wasm (single thread) is safe but we need to Clone Arc if needed
    // Actually, Interpreter holds Arc<StdMutex<SymbolTable>>, so we can clone Interpreter comfortably?
    // Interpreter struct does not implement Clone, but fields are Arcs.
    // Let's rely on stored instance.
    
    let interpreter_opt = {
        let guard = GLOBAL_INTERPRETER.lock().unwrap();
        // interpreting logic needs async. But handle_event is async.
        // We cannot keep the lock while awaiting.
        // But Interpreter doesn't need mutable self for execute_verb? 
        // execute_verb(&self)
        // Wait, execute_verb is on Interpreter, but we need to modify SymbolTable potentially.
        // Interpreter holds Arcs, so referencing it is fine.
        // But we can't move it out if we want to reuse it.
        // We need to implement Clone for Interpreter or reference it.
        // Since we can't easily reference from static mutex across await (lifetime issue),
        // we should Clone the Interpreter (cheap, just Arcs).
        // Let's modify Interpreter to derive Clone.
        guard.clone()
    };

    if let Some(interpreter) = interpreter_opt {
         let handler_body = {
             let handlers = interpreter.event_handlers.lock().unwrap();
             handlers.get(&(target.clone(), event_type.clone())).cloned()
         };

         if let Some(body) = handler_body {
             log::info!("[AGN] Handling event: {} on {}", event_type, target);
             
             // Execute handler body
             // Need to push context? Maybe.
             // If handler uses "self", we should push target to context stack.
             {
                 let mut stack = interpreter.context_stack.lock().unwrap();
                 stack.push(target.clone());
             }
             
             interpreter.execute_statements(&body).await;
             
             {
                 let mut stack = interpreter.context_stack.lock().unwrap();
                 stack.pop();
             }
         } else {
             log::warn!("[AGN] No handler found for event: {} on {}", event_type, target);
         }
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn main_js() {
    // console_log init moved to run_script or here
}

// ============================================================
// Eeyo WASM API: PWAから呼び出されるエントリーポイント
// ============================================================

/// Eeyo初期化
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn eeyo_init() -> Result<(), JsValue> {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Info).ok();
    log::info!("[Eeyo WASM] 初期化完了");
    Ok(())
}

/// ビーコン発信開始
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn eeyo_start_beacon(beacon_type: &str) -> Result<(), JsValue> {
    log::info!("[Eeyo WASM] ビーコン発信開始: {}", beacon_type);
    
    // P2P APIを呼び出し
    crate::p2p::agn_broadcast_beacon(beacon_type, None)
        .await
        .map_err(|e| JsValue::from_str(&e))?;
    
    Ok(())
}

/// ビーコン発信停止
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn eeyo_stop_beacon() -> Result<(), JsValue> {
    log::info!("[Eeyo WASM] ビーコン発信停止");
    // P2Pマネージャの停止はグローバルインスタンス経由
    Ok(())
}

/// 近くのピアを検索（JSON形式で返す）
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn eeyo_search_nearby(max_distance: f64) -> Result<String, JsValue> {
    log::info!("[Eeyo WASM] 近くのピアを検索: {}m以内", max_distance);
    
    let peers = crate::p2p::agn_spatial_search(max_distance, &[]).await;
    
    // JSON形式に変換
    let peers_json: Vec<serde_json::Value> = peers.iter().map(|p| {
        serde_json::json!({
            "id": p.peer_id,
            "distance": p.estimated_distance,
            "beacon_type": format!("{:?}", p.beacon_type),
            "toku_score": p.toku_score,
            "rssi": p.rssi
        })
    }).collect();
    
    serde_json::to_string(&peers_json)
        .map_err(|e| JsValue::from_str(&format!("JSON変換エラー: {}", e)))
}

/// 徳スコアを取得
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn eeyo_get_toku(user_id: &str) -> u32 {
    crate::p2p::agn_get_toku(user_id)
}

/// 徳スコアを加算
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn eeyo_add_toku(user_id: &str, amount: u32) {
    log::info!("[Eeyo WASM] 徳加算: {} +{}", user_id, amount);
    crate::p2p::agn_add_toku(user_id, amount);
}

/// ピアに通知を送信
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn eeyo_notify_peer(peer_id: &str, message: &str) -> Result<(), JsValue> {
    log::info!("[Eeyo WASM] 通知送信: {} → {}", peer_id, message);
    
    crate::p2p::agn_notify_peer(peer_id, message)
        .await
        .map_err(|e| JsValue::from_str(&e))?;
    
    Ok(())
}

/// ビーコンパケットを生成（デバッグ用）
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn eeyo_create_beacon_packet(beacon_type: &str, toku_score: u16, user_id: &str) -> Vec<u8> {
    use crate::p2p::{BeaconType, EeyoBeaconPacket, TokuManager};
    
    let bt = match beacon_type {
        "idle" | "暇" => BeaconType::Idle,
        "need_help" | "助けて" => BeaconType::NeedHelp,
        "touring" | "観光中" => BeaconType::Touring,
        _ => BeaconType::Custom(0x00),
    };
    
    let uid_hash = TokuManager::hash_user_id(user_id);
    let packet = EeyoBeaconPacket::new(bt, toku_score, uid_hash);
    
    packet.to_bytes().to_vec()
}

/// セキュアなビーコンパケットを生成 (Phase 17)
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn eeyo_create_secure_beacon_packet(beacon_type: &str, toku_score: u16) -> Vec<u8> {
    use crate::p2p::{BeaconType, EeyoSecurePacket, SECURITY_CONTEXT};
    
    let bt = match beacon_type {
        "idle" | "暇" => BeaconType::Idle,
        "need_help" | "助けて" => BeaconType::NeedHelp,
        "touring" | "観光中" => BeaconType::Touring,
        _ => BeaconType::Custom(0x00),
    };
    
    let context = SECURITY_CONTEXT.lock().unwrap();
    let public_key = context.verifying_key.to_bytes();
    let signing_key = &context.signing_key;
    
    let mut packet = EeyoSecurePacket::new(bt, toku_score, &public_key, signing_key);
    
    packet.to_bytes()
}

/// セキュアなビーコンパケット（バイト列）をパースしてJSONで返す
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn eeyo_parse_secure_packet(packet_bytes: &[u8]) -> Option<String> {
    use crate::p2p::EeyoSecurePacket;
    if let Some(packet) = EeyoSecurePacket::from_bytes(packet_bytes) {
        // [u8; 64] などがSerdeで直接扱いにくいため、手動でJSON化
        let json = serde_json::json!({
            "beacon_type": format!("{:?}", packet.beacon_type),
            "toku_score": packet.toku_score,
            "nonce": packet.nonce,
            "timestamp": packet.timestamp,
            "sender_public_key": hex::encode(packet.sender_public_key),
            "signature": hex::encode(packet.signature),
        });
        Some(json.to_string())
    } else {
        None
    }
}

// ------------------------------------------------------------
// 徳フィード (Social Toku Feed) API
// ------------------------------------------------------------

/// フィードイベントをシミュレーション（デモ用）
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn eeyo_simulate_gossip() -> Result<String, JsValue> {
    let manager = crate::p2p::P2PManager::new();
    let events = manager.simulate_incoming_gossip();
    
    // イベントフック: P2Pイベントをインタプリタに通知
    // Lock and clone interpreter
    let interpreter_opt = {
        let guard = GLOBAL_INTERPRETER.lock().unwrap();
        guard.clone()
    };

    if let Some(interpreter) = interpreter_opt {
        for event in &events {
            let type_str = match event.event_type {
                crate::p2p::SocialEventType::HelpGiven => "HelpGiven",
                crate::p2p::SocialEventType::ThankYou => "ThankYou",
                crate::p2p::SocialEventType::PassedBy => "PassedBy",
                crate::p2p::SocialEventType::TokuSent { .. } => "TokuSent",
            };
            
            let interpreter = interpreter.clone();
            let from_id = event.actor_id.clone();
            let to_id = event.target_id.clone();
            let type_string = type_str.to_string();
            
            // Spawn async task to handle event
            wasm_bindgen_futures::spawn_local(async move {
                interpreter.trigger_event(&type_string, &from_id, &to_id).await;
            });
        }
    }
    
    serde_json::to_string(&events)
        .map_err(|e| JsValue::from_str(&format!("JSON変換エラー: {}", e)))
}

/// ソーシャルイベントを発行
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn eeyo_publish_social_event(
    event_type_str: &str, 
    target_id: &str, 
    message: Option<String>
) -> Result<String, JsValue> {
    use crate::p2p::{SocialTokuEvent, SocialEventType, P2PManager};
    
    let event_type = match event_type_str {
        "help_given" | "助けた" => SocialEventType::HelpGiven,
        "thank_you" | "ありがとう" => SocialEventType::ThankYou,
        "passed_by" | "すれ違い" => SocialEventType::PassedBy,
        _ => return Err(JsValue::from_str("不明なイベントタイプ")),
    };
    
    let mut event = SocialTokuEvent::new("current_user", target_id, event_type);
    if let Some(msg) = message {
        event = event.with_message(&msg);
    }
    
    // イベントをブロードキャスト（シミュレーション）
    let manager = P2PManager::new();
    manager.broadcast_social_event(event.clone());
    
    serde_json::to_string(&event)
        .map_err(|e| JsValue::from_str(&format!("JSON変換エラー: {}", e)))
}

/// 絆情報を取得（JSON形式）
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn eeyo_get_bond(from: &str, to: &str) -> Result<String, JsValue> {
    let rel = crate::p2p::agn_get_bond(from, to);
    serde_json::to_string(&rel)
        .map_err(|e| JsValue::from_str(&format!("JSON変換エラー: {}", e)))
}

/// 絆を深める
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn eeyo_deepen_bond(from: &str, to: &str, amount: u32) {
    log::info!("[Eeyo WASM] 絆深化: {} ⇔ {} (+{})", from, to, amount);
    crate::p2p::agn_deepen_bond(from, to, amount);
}

/// 絆があるか確認
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn eeyo_has_bond(from: &str, to: &str) -> bool {
    crate::p2p::agn_has_bond(from, to)
}
