use crate::log_types::{BackendAddr, LoggableTime};
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{fmt::Display, net::SocketAddr};

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, PartialOrd, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum BackendStatus {
    /// The backend has been scheduled to a drone, but has not yet been acknowledged.
    /// This status is only assigned by the controller; the drone will never assign it by definition.
    Scheduled,

    /// The backend has been assigned to a drone, which is now responsible for loading its image.
    Loading,

    /// Telling Docker to start the container.
    Starting,

    /// Wait for the backend to be ready to accept connections.
    Waiting,

    /// The backend is listening for connections.
    Ready,

    /// The backend has been sent a SIGTERM, either because we sent it or the user did,
    /// and we are waiting for it to exit.
    /// Proxies should stop sending traffic to it, but we should not yet release the key.
    Terminating,

    /// The backend has been sent a SIGKILL, either because the user sent a hard termination
    /// request or the lock was past the hard-termination deadline.
    #[serde(rename = "hard-terminating")]
    HardTerminating,

    /// The backend has exited or been swept.
    Terminated,
}

impl BackendStatus {
    /// Returns an integer representation of the status. This is meant to be an
    /// opaque value that can be used for determining if one status comes before
    /// another in the backend lifecycle.
    /// Gaps are intentionally left in these values to provide room for future
    /// statuses.
    pub fn as_int(&self) -> i32 {
        match self {
            BackendStatus::Scheduled => 10,
            BackendStatus::Loading => 20,
            BackendStatus::Starting => 30,
            BackendStatus::Waiting => 40,
            BackendStatus::Ready => 50,
            BackendStatus::Terminating => 60,
            BackendStatus::HardTerminating => 65,
            BackendStatus::Terminated => 70,
        }
    }
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TerminationKind {
    Soft,
    Hard,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum BackendState {
    Scheduled,
    Loading,
    Starting,
    Waiting {
        address: BackendAddr,
    },
    Ready {
        address: BackendAddr,
    },
    Terminating {
        /// Last status before either soft or hard termination.
        last_status: BackendStatus,
        #[deprecated(note = "Use HardTerminating instead")]
        termination: TerminationKind,
        reason: TerminationReason,
    },
    #[serde(rename = "hard-terminating")]
    HardTerminating {
        /// Last status before either soft or hard termination.
        last_status: BackendStatus,
        reason: TerminationReason,
    },
    Terminated {
        last_status: BackendStatus,
        termination: Option<TerminationKind>,
        reason: Option<TerminationReason>,
        exit_code: Option<i32>,
    },
}

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TerminationReason {
    Swept,
    External,
    KeyExpired,
    Lost,
    StartupTimeout,
    InternalError,
}

#[derive(Clone, Debug, Serialize, Deserialize, thiserror::Error)]
pub enum BackendError {
    #[error("Timeout waiting for backend to start")]
    StartupTimeout,
    #[error("Container exited during startup (exit_code={0:?})")]
    ContainerExited(Option<i32>),
    #[error("{0}")]
    Other(String),
}

impl BackendState {
    pub fn address(&self) -> Option<BackendAddr> {
        match self {
            BackendState::Waiting { address } => Some(*address),
            BackendState::Ready { address } => Some(*address),
            _ => None,
        }
    }

    pub fn status(&self) -> BackendStatus {
        match self {
            BackendState::Scheduled => BackendStatus::Scheduled,
            BackendState::Loading => BackendStatus::Loading,
            BackendState::Starting => BackendStatus::Starting,
            BackendState::Waiting { .. } => BackendStatus::Waiting,
            BackendState::Ready { .. } => BackendStatus::Ready,
            BackendState::Terminating { .. } => BackendStatus::Terminating,
            BackendState::HardTerminating { .. } => BackendStatus::HardTerminating,
            BackendState::Terminated { .. } => BackendStatus::Terminated,
        }
    }

    pub fn status_int(&self) -> i32 {
        self.status().as_int()
    }

    pub fn to_loading(&self) -> BackendState {
        BackendState::Loading
    }

    pub fn to_starting(&self) -> BackendState {
        BackendState::Starting
    }

    pub fn to_waiting(&self, address: SocketAddr) -> BackendState {
        BackendState::Waiting {
            address: BackendAddr(address),
        }
    }

    pub fn to_ready(&self, address: BackendAddr) -> BackendState {
        BackendState::Ready { address }
    }

    pub fn to_terminating(&self, reason: TerminationReason) -> BackendState {
        if self.status() >= BackendStatus::Terminating {
            tracing::warn!(?reason, state=?self, "to_terminating called on backend in later state.");
            return self.clone();
        }

        BackendState::Terminating {
            last_status: self.status(),
            termination: TerminationKind::Soft,
            reason,
        }
    }

    pub fn to_hard_terminating(&self, reason: TerminationReason) -> BackendState {
        if self.status() >= BackendStatus::HardTerminating {
            tracing::warn!(?reason, state=?self, "to_hard_terminating called on backend in later state.");
            return self.clone();
        }

        match self {
            BackendState::Terminating { last_status, .. } => BackendState::HardTerminating {
                last_status: *last_status,
                reason,
            },
            _ => BackendState::HardTerminating {
                last_status: self.status(),
                reason,
            },
        }
    }

    pub fn to_terminated(&self, exit_code: Option<i32>) -> BackendState {
        match self {
            BackendState::Terminated { .. } => {
                tracing::warn!(?exit_code, state=?self, "to_terminated called on terminated backend");
                self.clone()
            }
            BackendState::HardTerminating {
                last_status,
                reason,
            } => BackendState::Terminated {
                last_status: *last_status,
                termination: Some(TerminationKind::Hard),
                reason: Some(*reason),
                exit_code,
            },
            #[allow(deprecated)]
            BackendState::Terminating {
                last_status,
                termination,
                reason,
            } => BackendState::Terminated {
                last_status: *last_status,
                termination: Some(*termination),
                reason: Some(*reason),
                exit_code,
            },
            _ => BackendState::Terminated {
                last_status: self.status(),
                termination: None,
                reason: None,
                exit_code,
            },
        }
    }
}

impl Default for BackendState {
    fn default() -> Self {
        Self::Scheduled
    }
}

impl TryFrom<String> for BackendStatus {
    type Error = serde_json::Error;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        serde_json::from_value(Value::String(s))
    }
}

impl Display for BackendStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let result = serde_json::to_value(self);
        match result {
            Ok(Value::String(v)) => write!(f, "{}", v),
            _ => unreachable!(),
        }
    }
}

/// A timestamped representation of a backend's status, along with
/// termination information. This is used for public-facing endpoints.
/// It does not include the backend's address, which is only available
/// to the controller.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct BackendStatusStreamEntry {
    pub status: BackendStatus,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub termination_reason: Option<TerminationReason>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub termination_kind: Option<TerminationKind>,

    /// Whether the process exited with an error. None if the process
    /// is still running.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_error: Option<bool>,

    pub time: LoggableTime,
}

impl BackendStatusStreamEntry {
    pub fn from_state(state: BackendState, timestamp: DateTime<Utc>) -> Self {
        let termination_reason = match state {
            BackendState::Terminated { reason, .. } => reason,
            BackendState::Terminating { reason, .. } => Some(reason),
            BackendState::HardTerminating { reason, .. } => Some(reason),
            _ => None,
        };

        let termination_kind = match state {
            BackendState::Terminated { termination, .. } => termination,
            BackendState::Terminating { .. } => Some(TerminationKind::Soft),
            BackendState::HardTerminating { .. } => Some(TerminationKind::Hard),
            _ => None,
        };

        let exit_error = match state {
            BackendState::Terminated {
                exit_code: Some(d), ..
            } if d != 0 => Some(true),
            BackendState::Terminated { .. } => Some(false),
            _ => None,
        };

        Self {
            status: state.status(),
            termination_reason,
            termination_kind,
            exit_error,
            time: LoggableTime(timestamp),
        }
    }
}
