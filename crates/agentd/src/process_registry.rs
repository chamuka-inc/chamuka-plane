use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, ChildStdin};
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct ProcessRegistry {
    processes: Arc<Mutex<HashMap<String, ProcessEntry>>>,
}

struct ProcessEntry {
    child: Child,
    stdin: Option<ChildStdin>,
}

pub enum ProcessStatus {
    Running,
    Exited(i32),
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

    pub async fn insert(&self, cmd_id: String, mut child: Child) {
        let stdin = child.stdin.take();
        let mut procs = self.processes.lock().await;
        procs.insert(cmd_id, ProcessEntry { child, stdin });
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

    pub async fn status(&self, cmd_id: &str) -> Result<ProcessStatus, SignalError> {
        let mut procs = self.processes.lock().await;
        let entry = procs.get_mut(cmd_id).ok_or(SignalError::NotFound)?;

        match entry.child.try_wait() {
            Ok(Some(exit)) => Ok(ProcessStatus::Exited(exit.code().unwrap_or(-1))),
            Ok(None) => Ok(ProcessStatus::Running),
            Err(e) => Err(SignalError::OsError(e)),
        }
    }

    pub async fn write_stdin(&self, cmd_id: &str, data: &[u8]) -> Result<(), StdinError> {
        let mut procs = self.processes.lock().await;
        let entry = procs.get_mut(cmd_id).ok_or(StdinError::NotFound)?;
        let stdin = entry.stdin.as_mut().ok_or(StdinError::NoStdin)?;
        stdin.write_all(data).await.map_err(StdinError::IoError)
    }

    pub async fn remove(&self, cmd_id: &str) -> Option<Child> {
        let mut procs = self.processes.lock().await;
        procs.remove(cmd_id).map(|e| e.child)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SignalError {
    #[error("process not found")]
    NotFound,
    #[error("process already exited")]
    ProcessExited,
    #[error("signal delivery failed: {0}")]
    OsError(#[from] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum StdinError {
    #[error("process not found")]
    NotFound,
    #[error("process has no stdin pipe")]
    NoStdin,
    #[error("write failed: {0}")]
    IoError(std::io::Error),
}
