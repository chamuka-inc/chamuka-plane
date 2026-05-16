use std::sync::{Arc, Mutex};

use crate::protocol::MessageToDrone;

#[derive(Debug, Clone)]
pub struct MockDrone {
    received: Arc<Mutex<Vec<MessageToDrone>>>,
}

impl Default for MockDrone {
    fn default() -> Self {
        Self::new()
    }
}

impl MockDrone {
    pub fn new() -> Self {
        Self {
            received: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn record(&self, msg: MessageToDrone) {
        self.received
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .push(msg);
    }

    pub fn received(&self) -> Vec<MessageToDrone> {
        self.received
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }

    pub fn received_count(&self) -> usize {
        self.received
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .len()
    }

    pub fn clear(&self) {
        self.received
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clear();
    }
}
