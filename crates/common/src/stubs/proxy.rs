use std::sync::{Arc, Mutex};

use crate::protocol::MessageToProxy;

#[derive(Debug, Clone)]
pub struct MockProxy {
    received: Arc<Mutex<Vec<MessageToProxy>>>,
}

impl Default for MockProxy {
    fn default() -> Self {
        Self::new()
    }
}

impl MockProxy {
    pub fn new() -> Self {
        Self {
            received: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn record(&self, msg: MessageToProxy) {
        self.received
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .push(msg);
    }

    pub fn received(&self) -> Vec<MessageToProxy> {
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
