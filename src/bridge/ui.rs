use crate::interpreter::RuntimeMessage;
use crate::p2p::SocialTokuEvent;

pub trait UIManager: Send + Sync {
    fn update_feed(&self, events: Vec<SocialTokuEvent>);
    fn notify(&self, message: &str);
    fn send_runtime_message(&self, msg: RuntimeMessage);
}
