use serde::Serialize;
use tabled::Tabled;

#[derive(Debug, Tabled, Serialize)]
pub struct DroneRow {
    pub name: String,
    pub ready: bool,
    pub draining: bool,
    pub backends: u32,
    pub version: String,
    pub heartbeat_age_s: String,
}

impl DroneRow {
    pub fn from_state(state: &common::types::DroneState) -> Self {
        Self {
            name: state.node.name.to_string(),
            ready: state.ready,
            draining: state.draining,
            backends: state.backend_count,
            version: state.node.plane_version.clone(),
            heartbeat_age_s: format!("{}s", state.last_heartbeat_age.num_seconds()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::types::{ControllerName, DroneName, DroneState, Name, NodeState};

    #[test]
    fn drone_row_from_state() {
        let drone_name = DroneName::new_random();
        let drone_name_str = drone_name.to_string();
        let state = DroneState {
            ready: true,
            draining: false,
            last_heartbeat_age: chrono::TimeDelta::seconds(5),
            backend_count: 3,
            node: NodeState {
                name: common::types::AnyNodeName::Drone(drone_name),
                plane_version: "0.5.0".to_string(),
                plane_hash: "abc123".to_string(),
                controller: ControllerName::new_random(),
                controller_heartbeat_age: chrono::TimeDelta::seconds(2),
            },
        };
        let row = DroneRow::from_state(&state);
        assert_eq!(row.name, drone_name_str);
        assert!(row.ready);
        assert!(!row.draining);
        assert_eq!(row.backends, 3);
        assert_eq!(row.version, "0.5.0");
        assert_eq!(row.heartbeat_age_s, "5s");
    }
}
