use crate::p2p::{DetectedPeer, Relationship, SocialTokuEvent};
use async_trait::async_trait;

#[async_trait]
pub trait P2PBridge: Send + Sync {
    // Beacon / Search
    async fn broadcast_beacon(&self, beacon_type: &str, duration: Option<u64>);
    async fn get_nearby_peers(&self, max_distance: f64) -> Vec<DetectedPeer>;
    async fn spatial_search(&self, max_distance: f64, filters: &[(String, String)]) -> Vec<DetectedPeer>;
    async fn notify_peer(&self, peer_id: &str, message: &str) -> Result<(), String>;
    
    // Toku Management
    fn get_toku(&self, user_id: &str) -> u32;
    fn add_toku(&self, user_id: &str, amount: u32);
    fn subtract_toku(&self, user_id: &str, amount: u32);
    
    // Bond Management
    fn get_bond(&self, from: &str, to: &str) -> Relationship;
    fn deepen_bond(&self, from: &str, to: &str, amount: u32);
    fn has_bond(&self, from: &str, to: &str) -> bool;
    fn set_bond_status(&self, from: &str, to: &str, status: &str);
    
    // Social Feed
    async fn get_all_feed_events(&self) -> Vec<SocialTokuEvent>;
    async fn get_feed_event(&self, id: &str) -> Option<SocialTokuEvent>;
    fn inject_feed_event(&self, event: SocialTokuEvent); // For testing
}
