use std::sync::{Arc, Mutex, MutexGuard};

#[derive(Debug, Clone)]
pub struct MessageRecorder<T> {
    received: Arc<Mutex<Vec<T>>>,
}

impl<T> Default for MessageRecorder<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> MessageRecorder<T> {
    pub fn new() -> Self {
        Self {
            received: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn lock_inner(&self) -> MutexGuard<'_, Vec<T>> {
        self.received.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn record(&self, msg: T) {
        self.lock_inner().push(msg);
    }

    pub fn received_count(&self) -> usize {
        self.lock_inner().len()
    }

    pub fn clear(&self) {
        self.lock_inner().clear();
    }
}

impl<T: Clone> MessageRecorder<T> {
    pub fn received(&self) -> Vec<T> {
        self.lock_inner().clone()
    }
}
