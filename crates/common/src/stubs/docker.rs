use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct ContainerCreateRequest {
    pub image: String,
    pub name: String,
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerState {
    Created,
    Running,
    Stopped,
}

#[derive(Debug, Clone)]
struct Container {
    request: ContainerCreateRequest,
    state: ContainerState,
}

#[derive(Debug, Clone)]
pub struct MockDockerSocket {
    containers: Arc<Mutex<Vec<Container>>>,
}

impl Default for MockDockerSocket {
    fn default() -> Self {
        Self::new()
    }
}

impl MockDockerSocket {
    pub fn new() -> Self {
        Self {
            containers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn create_container(&self, req: ContainerCreateRequest) -> String {
        let id = format!("mock-container-{}", self.container_count());
        let container = Container {
            request: req,
            state: ContainerState::Created,
        };
        self.containers
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .push(container);
        id
    }

    pub fn start_container(&self, index: usize) -> bool {
        let mut containers = self.containers.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(c) = containers.get_mut(index) {
            c.state = ContainerState::Running;
            true
        } else {
            false
        }
    }

    pub fn stop_container(&self, index: usize) -> bool {
        let mut containers = self.containers.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(c) = containers.get_mut(index) {
            c.state = ContainerState::Stopped;
            true
        } else {
            false
        }
    }

    pub fn container_count(&self) -> usize {
        self.containers
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .len()
    }

    pub fn container_state(&self, index: usize) -> Option<ContainerState> {
        self.containers
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get(index)
            .map(|c| c.state)
    }

    pub fn container_image(&self, index: usize) -> Option<String> {
        self.containers
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .get(index)
            .map(|c| c.request.image.clone())
    }

    pub fn clear(&self) {
        self.containers
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clear();
    }
}
