use std::sync::{Arc, Mutex};

use crate::protocol::MessageFromDrone;

#[derive(Debug, Clone)]
pub struct MockController {
    received: Arc<Mutex<Vec<MessageFromDrone>>>,
}

impl Default for MockController {
    fn default() -> Self {
        Self::new()
    }
}

impl MockController {
    pub fn new() -> Self {
        Self {
            received: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn record(&self, msg: MessageFromDrone) {
        self.received
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .push(msg);
    }

    pub fn received(&self) -> Vec<MessageFromDrone> {
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
