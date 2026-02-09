use crate::bridge::{P2PBridge, UIManager};
use crate::p2p::{DetectedPeer, Relationship, SocialTokuEvent};
use crate::interpreter::RuntimeMessage;
use async_trait::async_trait;
use std::sync::Arc;

pub struct StdP2PBridge;

#[async_trait]
impl P2PBridge for StdP2PBridge {
    async fn broadcast_beacon(&self, beacon_type: &str, duration: Option<u64>) {
        crate::p2p::agn_broadcast_beacon(beacon_type, duration).await.ok();
    }
    async fn get_nearby_peers(&self, max_distance: f64) -> Vec<DetectedPeer> {
        crate::p2p::agn_spatial_search(max_distance, &[]).await
    }
    async fn spatial_search(&self, max_distance: f64, filters: &[(String, String)]) -> Vec<DetectedPeer> {
        crate::p2p::agn_spatial_search(max_distance, filters).await
    }
    async fn notify_peer(&self, peer_id: &str, message: &str) -> Result<(), String> {
        crate::p2p::agn_notify_peer(peer_id, message).await
    }
    
    // Toku
    fn get_toku(&self, user_id: &str) -> u32 {
        crate::p2p::agn_get_toku(user_id)
    }
    fn add_toku(&self, user_id: &str, amount: u32) {
        crate::p2p::agn_add_toku(user_id, amount);
    }
    fn subtract_toku(&self, user_id: &str, amount: u32) {
        crate::p2p::agn_subtract_toku(user_id, amount);
    }
    
    // Bond
    fn get_bond(&self, from: &str, to: &str) -> Relationship {
        crate::p2p::agn_get_bond(from, to)
    }
    fn deepen_bond(&self, from: &str, to: &str, amount: u32) {
        crate::p2p::agn_deepen_bond(from, to, amount);
    }
    fn has_bond(&self, from: &str, to: &str) -> bool {
        crate::p2p::agn_has_bond(from, to)
    }
    fn set_bond_status(&self, _from: &str, _to: &str, _status: &str) {
        // Future: crate::p2p::agn_set_bond_status(from, to, status);
    }
    
    // Feed
    async fn get_all_feed_events(&self) -> Vec<SocialTokuEvent> {
        crate::p2p::agn_get_all_feed_events().await
    }
    async fn get_feed_event(&self, id: &str) -> Option<SocialTokuEvent> {
        crate::p2p::agn_get_feed_event(id).await
    }
    fn inject_feed_event(&self, event: SocialTokuEvent) {
        crate::p2p::agn_inject_feed_event(event);
    }
}

pub struct StdUIManager;

impl UIManager for StdUIManager {
    fn update_feed(&self, _events: Vec<SocialTokuEvent>) {
        // This will be called from Interpreter.
        // For now, we manually trigger the update logic if needed, 
        // but often the Interpreter itself will decide when to call this.
        // We'll move the heavy logic here later.
    }
    fn notify(&self, message: &str) {
        self.send_runtime_message(RuntimeMessage::String(message.to_string()));
    }
    fn send_runtime_message(&self, msg: RuntimeMessage) {
        if let Some(sender) = crate::interpreter::SCREEN_CHANNEL.lock().unwrap().as_ref() {
            sender.send(msg).ok();
        }
    }
}
