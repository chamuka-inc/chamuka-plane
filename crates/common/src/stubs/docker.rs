use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

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

    fn lock_inner(&self) -> MutexGuard<'_, Vec<Container>> {
        self.containers.lock().unwrap_or_else(|p| p.into_inner())
    }

    pub fn create_container(&self, req: ContainerCreateRequest) -> String {
        let mut containers = self.lock_inner();
        let id = format!("mock-container-{}", containers.len());
        containers.push(Container {
            request: req,
            state: ContainerState::Created,
        });
        id
    }

    pub fn start_container(&self, index: usize) -> bool {
        let mut containers = self.lock_inner();
        if let Some(c) = containers.get_mut(index) {
            c.state = ContainerState::Running;
            true
        } else {
            false
        }
    }

    pub fn stop_container(&self, index: usize) -> bool {
        let mut containers = self.lock_inner();
        if let Some(c) = containers.get_mut(index) {
            c.state = ContainerState::Stopped;
            true
        } else {
            false
        }
    }

    pub fn container_count(&self) -> usize {
        self.lock_inner().len()
    }

    pub fn container_state(&self, index: usize) -> Option<ContainerState> {
        self.lock_inner().get(index).map(|c| c.state)
    }

    pub fn container_image(&self, index: usize) -> Option<String> {
        self.lock_inner()
            .get(index)
            .map(|c| c.request.image.clone())
    }

    pub fn clear(&self) {
        self.lock_inner().clear();
    }
}
