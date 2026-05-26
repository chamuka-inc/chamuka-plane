mod screenshot;
mod validate;
mod xdotool;

#[cfg(feature = "desktop")]
use common::ext::agentd::{
    ComputerAction, ComputerActionRequest, ComputerActionResponse, ComputerOutput,
};
use thiserror::Error;

pub use screenshot::build_screenshot_args;
pub use validate::validate_request;
pub use xdotool::build_xdotool_args;

#[derive(Debug, Error)]
pub enum ComputerError {
    #[error("missing required field: {0}")]
    MissingField(&'static str),
    #[error("command execution failed: {0}")]
    ExecutionFailed(String),
    #[error("invalid coordinate: ({0}, {1})")]
    InvalidCoordinate(i32, i32),
}

#[cfg(feature = "desktop")]
async fn run_command(
    program: &str,
    args: &[String],
) -> Result<std::process::Output, ComputerError> {
    let result = tokio::process::Command::new(program)
        .args(args)
        .output()
        .await
        .map_err(|e| ComputerError::ExecutionFailed(e.to_string()))?;

    if !result.status.success() {
        return Err(ComputerError::ExecutionFailed(
            String::from_utf8_lossy(&result.stderr).to_string(),
        ));
    }
    Ok(result)
}

#[cfg(feature = "desktop")]
pub async fn execute(req: &ComputerActionRequest) -> Result<ComputerActionResponse, ComputerError> {
    validate_request(req)?;

    let output = match req.action {
        ComputerAction::Screenshot => {
            let args = build_screenshot_args(req);
            let result = run_command("scrot", &args).await?;

            use base64::Engine;
            let encoded = base64::engine::general_purpose::STANDARD.encode(&result.stdout);
            ComputerOutput::Image {
                base64: encoded,
                media_type: "image/png".to_string(),
            }
        }
        ComputerAction::CursorPosition => {
            let args = ["getmouselocation".to_string(), "--shell".to_string()];
            let result = run_command("xdotool", &args).await?;

            let stdout = String::from_utf8_lossy(&result.stdout);
            let (x, y) = xdotool::parse_xdotool_location(&stdout);
            ComputerOutput::Text {
                text: format!("{x},{y}"),
            }
        }
        ComputerAction::Wait => {
            let duration_ms = req.duration.unwrap_or(1000);
            tokio::time::sleep(std::time::Duration::from_millis(u64::from(duration_ms))).await;
            ComputerOutput::Ok
        }
        _ => {
            let args = build_xdotool_args(req);
            if args.is_empty() {
                return Ok(ComputerActionResponse {
                    output: ComputerOutput::Ok,
                });
            }
            run_command("xdotool", &args).await?;
            ComputerOutput::Ok
        }
    };

    Ok(ComputerActionResponse { output })
}
