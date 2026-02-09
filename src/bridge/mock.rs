use crate::bridge::{P2PBridge, UIManager};
use crate::p2p::{DetectedPeer, SocialTokuEvent, Relationship};
use crate::interpreter::RuntimeMessage;
use async_trait::async_trait;
use std::sync::Arc;

pub struct MockP2PBridge {
    pub toku_scores: std::sync::Mutex<std::collections::HashMap<String, u32>>,
    pub bonds: std::sync::Mutex<std::collections::HashMap<(String, String), Relationship>>,
    pub events: std::sync::Mutex<std::collections::HashMap<String, SocialTokuEvent>>,
}

impl MockP2PBridge {
    pub fn new() -> Self {
        Self {
            toku_scores: std::sync::Mutex::new(std::collections::HashMap::new()),
            bonds: std::sync::Mutex::new(std::collections::HashMap::new()),
            events: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

#[async_trait]
impl P2PBridge for MockP2PBridge {
    async fn broadcast_beacon(&self, _beacon_type: &str, _duration: Option<u64>) {}
    async fn get_nearby_peers(&self, _max_distance: f64) -> Vec<DetectedPeer> { Vec::new() }
    async fn spatial_search(&self, _max_distance: f64, _filters: &[(String, String)]) -> Vec<DetectedPeer> { Vec::new() }
    async fn notify_peer(&self, _peer_id: &str, _message: &str) -> Result<(), String> { Ok(()) }
    
    fn get_toku(&self, user_id: &str) -> u32 {
        *self.toku_scores.lock().unwrap().get(user_id).unwrap_or(&100)
    }
    fn add_toku(&self, user_id: &str, amount: u32) {
        let mut scores = self.toku_scores.lock().unwrap();
        let score = scores.entry(user_id.to_string()).or_insert(100);
        *score += amount;
    }
    fn subtract_toku(&self, user_id: &str, amount: u32) {
        let mut scores = self.toku_scores.lock().unwrap();
        let score = scores.entry(user_id.to_string()).or_insert(100);
        *score = score.saturating_sub(amount);
    }
    
    fn get_bond(&self, from: &str, to: &str) -> Relationship {
        self.bonds.lock().unwrap().get(&(from.to_string(), to.to_string())).cloned().unwrap_or_else(|| {
            Relationship {
                strength: 10,
                level: 1,
                last_interaction: 0,
                help_count: 0,
                first_met: 0,
                tags: Vec::new(),
            }
        })
    }
    fn deepen_bond(&self, from: &str, to: &str, amount: u32) {
        let mut bonds = self.bonds.lock().unwrap();
        let bond = bonds.entry((from.to_string(), to.to_string())).or_insert(Relationship {
            strength: 10,
            level: 1,
            last_interaction: 0,
            help_count: 0,
            first_met: 0,
            tags: Vec::new(),
        });
        bond.strength += amount;
    }
    fn has_bond(&self, from: &str, to: &str) -> bool {
        self.bonds.lock().unwrap().contains_key(&(from.to_string(), to.to_string()))
    }
    fn set_bond_status(&self, _from: &str, _to: &str, _status: &str) {}
    
    async fn get_all_feed_events(&self) -> Vec<SocialTokuEvent> {
        self.events.lock().unwrap().values().cloned().collect()
    }
    async fn get_feed_event(&self, id: &str) -> Option<SocialTokuEvent> {
        self.events.lock().unwrap().get(id).cloned()
    }
    fn inject_feed_event(&self, event: SocialTokuEvent) {
        self.events.lock().unwrap().insert(event.id.clone(), event);
    }
}

pub struct MockUIManager;

impl UIManager for MockUIManager {
    fn update_feed(&self, _events: Vec<SocialTokuEvent>) {}
    fn notify(&self, _message: &str) {}
    fn send_runtime_message(&self, _msg: RuntimeMessage) {}
}
