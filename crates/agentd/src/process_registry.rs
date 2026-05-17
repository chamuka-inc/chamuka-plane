use std::collections::HashMap;
use std::sync::Arc;
use tokio::process::Child;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct ProcessRegistry {
    processes: Arc<Mutex<HashMap<String, ProcessEntry>>>,
}

struct ProcessEntry {
    child: Child,
}

impl Default for ProcessRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessRegistry {
    pub fn new() -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn insert(&self, cmd_id: String, child: Child) {
        let mut procs = self.processes.lock().await;
        procs.insert(cmd_id, ProcessEntry { child });
    }

    pub async fn signal(&self, cmd_id: &str, signal: i32) -> Result<(), SignalError> {
        let mut procs = self.processes.lock().await;
        let entry = procs.get_mut(cmd_id).ok_or(SignalError::NotFound)?;

        let pid = entry.child.id().ok_or(SignalError::ProcessExited)?;

        // SAFETY: pid is obtained from a living child process we own;
        // signal is a valid Unix signal number passed by the caller.
        let result = unsafe { libc::kill(pid as i32, signal) };

        if result == 0 {
            Ok(())
        } else {
            Err(SignalError::OsError(std::io::Error::last_os_error()))
        }
    }

    pub async fn remove(&self, cmd_id: &str) -> Option<Child> {
        let mut procs = self.processes.lock().await;
        procs.remove(cmd_id).map(|e| e.child)
    }
}

#[derive(Debug)]
pub enum SignalError {
    NotFound,
    ProcessExited,
    OsError(std::io::Error),
}
