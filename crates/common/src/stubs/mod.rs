pub mod docker;
mod recorder;

pub use docker::MockDockerSocket;
pub use recorder::MessageRecorder;

use crate::protocol::{MessageFromDrone, MessageToDrone, MessageToProxy};

pub type MockController = MessageRecorder<MessageFromDrone>;
pub type MockDrone = MessageRecorder<MessageToDrone>;
pub type MockProxy = MessageRecorder<MessageToProxy>;
