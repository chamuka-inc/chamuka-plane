use common::ext::agentd::{ComputerAction, ComputerActionRequest, ScrollDirection};
#[cfg(feature = "desktop")]
use common::ext::agentd::{ComputerActionResponse, ComputerOutput};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ComputerError {
    #[error("missing required field: {0}")]
    MissingField(&'static str),
    #[error("command execution failed: {0}")]
    ExecutionFailed(String),
    #[error("invalid coordinate: ({0}, {1})")]
    InvalidCoordinate(i32, i32),
}

pub fn validate_request(req: &ComputerActionRequest) -> Result<(), ComputerError> {
    match req.action {
        ComputerAction::MouseMove
        | ComputerAction::LeftClick
        | ComputerAction::RightClick
        | ComputerAction::MiddleClick
        | ComputerAction::DoubleClick
        | ComputerAction::TripleClick
        | ComputerAction::LeftMouseDown
        | ComputerAction::LeftMouseUp => {
            let coord = req
                .coordinate
                .ok_or(ComputerError::MissingField("coordinate"))?;
            validate_coordinate(coord)?;
        }
        ComputerAction::LeftClickDrag => {
            let start = req
                .start_coordinate
                .ok_or(ComputerError::MissingField("start_coordinate"))?;
            let end = req
                .coordinate
                .ok_or(ComputerError::MissingField("coordinate"))?;
            validate_coordinate(start)?;
            validate_coordinate(end)?;
        }
        ComputerAction::Key | ComputerAction::Type | ComputerAction::HoldKey => {
            req.text
                .as_ref()
                .ok_or(ComputerError::MissingField("text"))?;
        }
        ComputerAction::Scroll => {
            req.scroll_direction
                .ok_or(ComputerError::MissingField("scroll_direction"))?;
            req.scroll_amount
                .ok_or(ComputerError::MissingField("scroll_amount"))?;
        }
        ComputerAction::Screenshot
        | ComputerAction::CursorPosition
        | ComputerAction::Wait
        | ComputerAction::Zoom => {}
    }
    Ok(())
}

fn validate_coordinate(coord: [i32; 2]) -> Result<(), ComputerError> {
    if coord[0] < 0 || coord[1] < 0 {
        return Err(ComputerError::InvalidCoordinate(coord[0], coord[1]));
    }
    Ok(())
}

pub fn build_xdotool_args(req: &ComputerActionRequest) -> Vec<String> {
    match req.action {
        ComputerAction::MouseMove => {
            let [x, y] = req.coordinate.unwrap_or([0, 0]);
            vec![
                "mousemove".to_string(),
                "--sync".to_string(),
                x.to_string(),
                y.to_string(),
            ]
        }
        ComputerAction::LeftClick => {
            let [x, y] = req.coordinate.unwrap_or([0, 0]);
            vec![
                "mousemove".to_string(),
                "--sync".to_string(),
                x.to_string(),
                y.to_string(),
                "click".to_string(),
                "1".to_string(),
            ]
        }
        ComputerAction::RightClick => {
            let [x, y] = req.coordinate.unwrap_or([0, 0]);
            vec![
                "mousemove".to_string(),
                "--sync".to_string(),
                x.to_string(),
                y.to_string(),
                "click".to_string(),
                "3".to_string(),
            ]
        }
        ComputerAction::MiddleClick => {
            let [x, y] = req.coordinate.unwrap_or([0, 0]);
            vec![
                "mousemove".to_string(),
                "--sync".to_string(),
                x.to_string(),
                y.to_string(),
                "click".to_string(),
                "2".to_string(),
            ]
        }
        ComputerAction::DoubleClick => {
            let [x, y] = req.coordinate.unwrap_or([0, 0]);
            vec![
                "mousemove".to_string(),
                "--sync".to_string(),
                x.to_string(),
                y.to_string(),
                "click".to_string(),
                "--repeat".to_string(),
                "2".to_string(),
                "1".to_string(),
            ]
        }
        ComputerAction::TripleClick => {
            let [x, y] = req.coordinate.unwrap_or([0, 0]);
            vec![
                "mousemove".to_string(),
                "--sync".to_string(),
                x.to_string(),
                y.to_string(),
                "click".to_string(),
                "--repeat".to_string(),
                "3".to_string(),
                "1".to_string(),
            ]
        }
        ComputerAction::LeftClickDrag => {
            let [sx, sy] = req.start_coordinate.unwrap_or([0, 0]);
            let [ex, ey] = req.coordinate.unwrap_or([0, 0]);
            vec![
                "mousemove".to_string(),
                "--sync".to_string(),
                sx.to_string(),
                sy.to_string(),
                "mousedown".to_string(),
                "1".to_string(),
                "mousemove".to_string(),
                "--sync".to_string(),
                ex.to_string(),
                ey.to_string(),
                "mouseup".to_string(),
                "1".to_string(),
            ]
        }
        ComputerAction::LeftMouseDown => {
            let [x, y] = req.coordinate.unwrap_or([0, 0]);
            vec![
                "mousemove".to_string(),
                "--sync".to_string(),
                x.to_string(),
                y.to_string(),
                "mousedown".to_string(),
                "1".to_string(),
            ]
        }
        ComputerAction::LeftMouseUp => {
            let [x, y] = req.coordinate.unwrap_or([0, 0]);
            vec![
                "mousemove".to_string(),
                "--sync".to_string(),
                x.to_string(),
                y.to_string(),
                "mouseup".to_string(),
                "1".to_string(),
            ]
        }
        ComputerAction::Key => {
            let text = req.text.as_deref().unwrap_or("");
            vec!["key".to_string(), "--".to_string(), text.to_string()]
        }
        ComputerAction::Type => {
            let text = req.text.as_deref().unwrap_or("");
            vec![
                "type".to_string(),
                "--clearmodifiers".to_string(),
                "--".to_string(),
                text.to_string(),
            ]
        }
        ComputerAction::HoldKey => {
            let text = req.text.as_deref().unwrap_or("");
            let duration_ms = req.duration.unwrap_or(100);
            vec![
                "keydown".to_string(),
                text.to_string(),
                "sleep".to_string(),
                format!("{}", f64::from(duration_ms) / 1000.0),
                "keyup".to_string(),
                text.to_string(),
            ]
        }
        ComputerAction::Scroll => {
            let [x, y] = req.coordinate.unwrap_or([0, 0]);
            let amount = req.scroll_amount.unwrap_or(3);
            let button = match req.scroll_direction.unwrap_or(ScrollDirection::Down) {
                ScrollDirection::Up => "4",
                ScrollDirection::Down => "5",
                ScrollDirection::Left => "6",
                ScrollDirection::Right => "7",
            };
            vec![
                "mousemove".to_string(),
                "--sync".to_string(),
                x.to_string(),
                y.to_string(),
                "click".to_string(),
                "--repeat".to_string(),
                amount.to_string(),
                button.to_string(),
            ]
        }
        ComputerAction::Wait => {
            vec![]
        }
        ComputerAction::Zoom => {
            vec![]
        }
        ComputerAction::Screenshot | ComputerAction::CursorPosition => {
            vec![]
        }
    }
}

pub fn build_screenshot_args(req: &ComputerActionRequest) -> Vec<String> {
    match req.region {
        Some([x, y, w, h]) => vec![
            "-a".to_string(),
            format!("{x},{y},{w},{h}"),
            "-o".to_string(),
            "/dev/stdout".to_string(),
        ],
        None => vec!["-o".to_string(), "/dev/stdout".to_string()],
    }
}

#[cfg(feature = "desktop")]
pub async fn execute(req: &ComputerActionRequest) -> Result<ComputerActionResponse, ComputerError> {
    validate_request(req)?;

    let output = match req.action {
        ComputerAction::Screenshot => {
            let args = build_screenshot_args(req);
            let result = tokio::process::Command::new("scrot")
                .args(&args)
                .output()
                .await
                .map_err(|e| ComputerError::ExecutionFailed(e.to_string()))?;

            if !result.status.success() {
                return Err(ComputerError::ExecutionFailed(
                    String::from_utf8_lossy(&result.stderr).to_string(),
                ));
            }

            use base64::Engine;
            let encoded = base64::engine::general_purpose::STANDARD.encode(&result.stdout);
            ComputerOutput::Image {
                base64: encoded,
                media_type: "image/png".to_string(),
            }
        }
        ComputerAction::CursorPosition => {
            let result = tokio::process::Command::new("xdotool")
                .args(["getmouselocation", "--shell"])
                .output()
                .await
                .map_err(|e| ComputerError::ExecutionFailed(e.to_string()))?;

            if !result.status.success() {
                return Err(ComputerError::ExecutionFailed(
                    String::from_utf8_lossy(&result.stderr).to_string(),
                ));
            }

            let stdout = String::from_utf8_lossy(&result.stdout);
            let (x, y) = parse_xdotool_location(&stdout);
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
            let result = tokio::process::Command::new("xdotool")
                .args(&args)
                .output()
                .await
                .map_err(|e| ComputerError::ExecutionFailed(e.to_string()))?;

            if !result.status.success() {
                return Err(ComputerError::ExecutionFailed(
                    String::from_utf8_lossy(&result.stderr).to_string(),
                ));
            }
            ComputerOutput::Ok
        }
    };

    Ok(ComputerActionResponse { output })
}

#[cfg(feature = "desktop")]
fn parse_xdotool_location(output: &str) -> (i32, i32) {
    let mut x = 0i32;
    let mut y = 0i32;
    for line in output.lines() {
        if let Some(val) = line.strip_prefix("X=") {
            x = val.parse().unwrap_or(0);
        } else if let Some(val) = line.strip_prefix("Y=") {
            y = val.parse().unwrap_or(0);
        }
    }
    (x, y)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn validate_left_click_requires_coordinate() {
        let req = ComputerActionRequest {
            action: ComputerAction::LeftClick,
            coordinate: None,
            start_coordinate: None,
            text: None,
            duration: None,
            region: None,
            scroll_direction: None,
            scroll_amount: None,
        };
        let result = validate_request(&req);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("coordinate"));
    }

    #[test]
    fn validate_left_click_rejects_negative_coordinate() {
        let req = ComputerActionRequest {
            action: ComputerAction::LeftClick,
            coordinate: Some([-1, 200]),
            start_coordinate: None,
            text: None,
            duration: None,
            region: None,
            scroll_direction: None,
            scroll_amount: None,
        };
        let result = validate_request(&req);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("invalid coordinate"));
    }

    #[test]
    fn validate_type_requires_text() {
        let req = ComputerActionRequest {
            action: ComputerAction::Type,
            coordinate: None,
            start_coordinate: None,
            text: None,
            duration: None,
            region: None,
            scroll_direction: None,
            scroll_amount: None,
        };
        let result = validate_request(&req);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("text"));
    }

    #[test]
    fn validate_scroll_requires_direction_and_amount() {
        let req = ComputerActionRequest {
            action: ComputerAction::Scroll,
            coordinate: Some([0, 0]),
            start_coordinate: None,
            text: None,
            duration: None,
            region: None,
            scroll_direction: None,
            scroll_amount: None,
        };
        let result = validate_request(&req);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("scroll_direction"));
    }

    #[test]
    fn validate_drag_requires_both_coordinates() {
        let req = ComputerActionRequest {
            action: ComputerAction::LeftClickDrag,
            coordinate: Some([100, 100]),
            start_coordinate: None,
            text: None,
            duration: None,
            region: None,
            scroll_direction: None,
            scroll_amount: None,
        };
        let result = validate_request(&req);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("start_coordinate"));
    }

    #[test]
    fn validate_screenshot_needs_nothing() {
        let req = ComputerActionRequest {
            action: ComputerAction::Screenshot,
            coordinate: None,
            start_coordinate: None,
            text: None,
            duration: None,
            region: None,
            scroll_direction: None,
            scroll_amount: None,
        };
        assert!(validate_request(&req).is_ok());
    }

    #[test]
    fn xdotool_args_left_click() {
        let req = ComputerActionRequest {
            action: ComputerAction::LeftClick,
            coordinate: Some([100, 200]),
            start_coordinate: None,
            text: None,
            duration: None,
            region: None,
            scroll_direction: None,
            scroll_amount: None,
        };
        let args = build_xdotool_args(&req);
        assert_eq!(
            args,
            vec!["mousemove", "--sync", "100", "200", "click", "1"]
        );
    }

    #[test]
    fn xdotool_args_right_click() {
        let req = ComputerActionRequest {
            action: ComputerAction::RightClick,
            coordinate: Some([50, 75]),
            start_coordinate: None,
            text: None,
            duration: None,
            region: None,
            scroll_direction: None,
            scroll_amount: None,
        };
        let args = build_xdotool_args(&req);
        assert_eq!(args, vec!["mousemove", "--sync", "50", "75", "click", "3"]);
    }

    #[test]
    fn xdotool_args_double_click() {
        let req = ComputerActionRequest {
            action: ComputerAction::DoubleClick,
            coordinate: Some([250, 350]),
            start_coordinate: None,
            text: None,
            duration: None,
            region: None,
            scroll_direction: None,
            scroll_amount: None,
        };
        let args = build_xdotool_args(&req);
        assert_eq!(
            args,
            vec![
                "mousemove",
                "--sync",
                "250",
                "350",
                "click",
                "--repeat",
                "2",
                "1"
            ]
        );
    }

    #[test]
    fn xdotool_args_triple_click() {
        let req = ComputerActionRequest {
            action: ComputerAction::TripleClick,
            coordinate: Some([10, 20]),
            start_coordinate: None,
            text: None,
            duration: None,
            region: None,
            scroll_direction: None,
            scroll_amount: None,
        };
        let args = build_xdotool_args(&req);
        assert_eq!(
            args,
            vec![
                "mousemove",
                "--sync",
                "10",
                "20",
                "click",
                "--repeat",
                "3",
                "1"
            ]
        );
    }

    #[test]
    fn xdotool_args_mouse_move() {
        let req = ComputerActionRequest {
            action: ComputerAction::MouseMove,
            coordinate: Some([500, 600]),
            start_coordinate: None,
            text: None,
            duration: None,
            region: None,
            scroll_direction: None,
            scroll_amount: None,
        };
        let args = build_xdotool_args(&req);
        assert_eq!(args, vec!["mousemove", "--sync", "500", "600"]);
    }

    #[test]
    fn xdotool_args_left_click_drag() {
        let req = ComputerActionRequest {
            action: ComputerAction::LeftClickDrag,
            coordinate: Some([300, 300]),
            start_coordinate: Some([100, 100]),
            text: None,
            duration: None,
            region: None,
            scroll_direction: None,
            scroll_amount: None,
        };
        let args = build_xdotool_args(&req);
        assert_eq!(
            args,
            vec![
                "mousemove",
                "--sync",
                "100",
                "100",
                "mousedown",
                "1",
                "mousemove",
                "--sync",
                "300",
                "300",
                "mouseup",
                "1"
            ]
        );
    }

    #[test]
    fn xdotool_args_key() {
        let req = ComputerActionRequest {
            action: ComputerAction::Key,
            coordinate: None,
            start_coordinate: None,
            text: Some("Return".to_string()),
            duration: None,
            region: None,
            scroll_direction: None,
            scroll_amount: None,
        };
        let args = build_xdotool_args(&req);
        assert_eq!(args, vec!["key", "--", "Return"]);
    }

    #[test]
    fn xdotool_args_type() {
        let req = ComputerActionRequest {
            action: ComputerAction::Type,
            coordinate: None,
            start_coordinate: None,
            text: Some("hello world".to_string()),
            duration: None,
            region: None,
            scroll_direction: None,
            scroll_amount: None,
        };
        let args = build_xdotool_args(&req);
        assert_eq!(args, vec!["type", "--clearmodifiers", "--", "hello world"]);
    }

    #[test]
    fn xdotool_args_hold_key() {
        let req = ComputerActionRequest {
            action: ComputerAction::HoldKey,
            coordinate: None,
            start_coordinate: None,
            text: Some("shift".to_string()),
            duration: Some(500),
            region: None,
            scroll_direction: None,
            scroll_amount: None,
        };
        let args = build_xdotool_args(&req);
        assert_eq!(
            args,
            vec!["keydown", "shift", "sleep", "0.5", "keyup", "shift"]
        );
    }

    #[test]
    fn xdotool_args_scroll_down() {
        let req = ComputerActionRequest {
            action: ComputerAction::Scroll,
            coordinate: Some([400, 300]),
            start_coordinate: None,
            text: None,
            duration: None,
            region: None,
            scroll_direction: Some(ScrollDirection::Down),
            scroll_amount: Some(3),
        };
        let args = build_xdotool_args(&req);
        assert_eq!(
            args,
            vec![
                "mousemove",
                "--sync",
                "400",
                "300",
                "click",
                "--repeat",
                "3",
                "5"
            ]
        );
    }

    #[test]
    fn xdotool_args_scroll_up() {
        let req = ComputerActionRequest {
            action: ComputerAction::Scroll,
            coordinate: Some([0, 0]),
            start_coordinate: None,
            text: None,
            duration: None,
            region: None,
            scroll_direction: Some(ScrollDirection::Up),
            scroll_amount: Some(5),
        };
        let args = build_xdotool_args(&req);
        assert_eq!(
            args,
            vec![
                "mousemove",
                "--sync",
                "0",
                "0",
                "click",
                "--repeat",
                "5",
                "4"
            ]
        );
    }

    #[test]
    fn xdotool_args_left_mouse_down() {
        let req = ComputerActionRequest {
            action: ComputerAction::LeftMouseDown,
            coordinate: Some([150, 250]),
            start_coordinate: None,
            text: None,
            duration: None,
            region: None,
            scroll_direction: None,
            scroll_amount: None,
        };
        let args = build_xdotool_args(&req);
        assert_eq!(
            args,
            vec!["mousemove", "--sync", "150", "250", "mousedown", "1"]
        );
    }

    #[test]
    fn xdotool_args_left_mouse_up() {
        let req = ComputerActionRequest {
            action: ComputerAction::LeftMouseUp,
            coordinate: Some([150, 250]),
            start_coordinate: None,
            text: None,
            duration: None,
            region: None,
            scroll_direction: None,
            scroll_amount: None,
        };
        let args = build_xdotool_args(&req);
        assert_eq!(
            args,
            vec!["mousemove", "--sync", "150", "250", "mouseup", "1"]
        );
    }

    #[test]
    fn screenshot_args_full_screen() {
        let req = ComputerActionRequest {
            action: ComputerAction::Screenshot,
            coordinate: None,
            start_coordinate: None,
            text: None,
            duration: None,
            region: None,
            scroll_direction: None,
            scroll_amount: None,
        };
        let args = build_screenshot_args(&req);
        assert_eq!(args, vec!["-o", "/dev/stdout"]);
    }

    #[test]
    fn screenshot_args_with_region() {
        let req = ComputerActionRequest {
            action: ComputerAction::Screenshot,
            coordinate: None,
            start_coordinate: None,
            text: None,
            duration: None,
            region: Some([10, 20, 100, 200]),
            scroll_direction: None,
            scroll_amount: None,
        };
        let args = build_screenshot_args(&req);
        assert_eq!(args, vec!["-a", "10,20,100,200", "-o", "/dev/stdout"]);
    }

    #[cfg(feature = "desktop")]
    #[test]
    fn parse_xdotool_location_output() {
        let output = "X=512\nY=384\nSCREEN=0\nWINDOW=12345\n";
        let (x, y) = parse_xdotool_location(output);
        assert_eq!(x, 512);
        assert_eq!(y, 384);
    }
}
