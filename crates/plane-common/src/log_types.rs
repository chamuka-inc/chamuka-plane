use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LoggableTime(#[serde(with = "chrono::serde::ts_milliseconds")] pub DateTime<Utc>);

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq, PartialOrd)]
pub struct BackendAddr(pub SocketAddr);
