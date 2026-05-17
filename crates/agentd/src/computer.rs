use common::ext::agentd::{ComputerAction, ComputerActionRequest, ScrollDirection};
#[cfg(feature = "desktop")]
use common::ext::agentd::{ComputerActionResponse, ComputerOutput};
use thiserror::Error;

const BTN_LEFT: &str = "1";
const BTN_MIDDLE: &str = "2";
const BTN_RIGHT: &str = "3";
const BTN_SCROLL_UP: &str = "4";
const BTN_SCROLL_DOWN: &str = "5";
const BTN_SCROLL_LEFT: &str = "6";
const BTN_SCROLL_RIGHT: &str = "7";

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
        | ComputerAction::LeftMouseUp
        | ComputerAction::Scroll => {
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
        ComputerAction::Screenshot
        | ComputerAction::CursorPosition
        | ComputerAction::Wait
        | ComputerAction::Zoom => {}
    }

    if matches!(req.action, ComputerAction::Scroll) {
        req.scroll_direction
            .ok_or(ComputerError::MissingField("scroll_direction"))?;
        req.scroll_amount
            .ok_or(ComputerError::MissingField("scroll_amount"))?;
    }

    Ok(())
}

fn validate_coordinate(coord: [i32; 2]) -> Result<(), ComputerError> {
    if coord[0] < 0 || coord[1] < 0 {
        return Err(ComputerError::InvalidCoordinate(coord[0], coord[1]));
    }
    Ok(())
}

fn mousemove_sync(x: i32, y: i32) -> Vec<String> {
    vec![
        "mousemove".to_string(),
        "--sync".to_string(),
        x.to_string(),
        y.to_string(),
    ]
}

fn click_at(coord: [i32; 2], button: &str) -> Vec<String> {
    let mut args = mousemove_sync(coord[0], coord[1]);
    args.push("click".to_string());
    args.push(button.to_string());
    args
}

fn click_repeat_at(coord: [i32; 2], repeat: i32, button: &str) -> Vec<String> {
    let mut args = mousemove_sync(coord[0], coord[1]);
    args.push("click".to_string());
    args.push("--repeat".to_string());
    args.push(repeat.to_string());
    args.push(button.to_string());
    args
}

fn mouse_button_at(coord: [i32; 2], action: &str, button: &str) -> Vec<String> {
    let mut args = mousemove_sync(coord[0], coord[1]);
    args.push(action.to_string());
    args.push(button.to_string());
    args
}

pub fn build_xdotool_args(req: &ComputerActionRequest) -> Vec<String> {
    let coord = req.coordinate.unwrap_or([0, 0]);

    match req.action {
        ComputerAction::MouseMove => mousemove_sync(coord[0], coord[1]),
        ComputerAction::LeftClick => click_at(coord, BTN_LEFT),
        ComputerAction::RightClick => click_at(coord, BTN_RIGHT),
        ComputerAction::MiddleClick => click_at(coord, BTN_MIDDLE),
        ComputerAction::DoubleClick => click_repeat_at(coord, 2, BTN_LEFT),
        ComputerAction::TripleClick => click_repeat_at(coord, 3, BTN_LEFT),
        ComputerAction::LeftMouseDown => mouse_button_at(coord, "mousedown", BTN_LEFT),
        ComputerAction::LeftMouseUp => mouse_button_at(coord, "mouseup", BTN_LEFT),
        ComputerAction::LeftClickDrag => {
            let start = req.start_coordinate.unwrap_or([0, 0]);
            let mut args = mouse_button_at(start, "mousedown", BTN_LEFT);
            args.extend(mouse_button_at(coord, "mouseup", BTN_LEFT));
            args
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
            let amount = req.scroll_amount.unwrap_or(3);
            let button = match req.scroll_direction.unwrap_or(ScrollDirection::Down) {
                ScrollDirection::Up => BTN_SCROLL_UP,
                ScrollDirection::Down => BTN_SCROLL_DOWN,
                ScrollDirection::Left => BTN_SCROLL_LEFT,
                ScrollDirection::Right => BTN_SCROLL_RIGHT,
            };
            click_repeat_at(coord, amount, button)
        }
        ComputerAction::Wait | ComputerAction::Zoom => vec![],
        ComputerAction::Screenshot | ComputerAction::CursorPosition => vec![],
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
            run_command("xdotool", &args).await?;
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

    fn req(action: ComputerAction) -> ComputerActionRequest {
        ComputerActionRequest {
            action,
            coordinate: None,
            start_coordinate: None,
            text: None,
            duration: None,
            region: None,
            scroll_direction: None,
            scroll_amount: None,
        }
    }

    #[test]
    fn validate_left_click_requires_coordinate() {
        let result = validate_request(&req(ComputerAction::LeftClick));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("coordinate"));
    }

    #[test]
    fn validate_left_click_rejects_negative_coordinate() {
        let r = ComputerActionRequest {
            coordinate: Some([-1, 200]),
            ..req(ComputerAction::LeftClick)
        };
        let result = validate_request(&r);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("invalid coordinate"));
    }

    #[test]
    fn validate_type_requires_text() {
        let result = validate_request(&req(ComputerAction::Type));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("text"));
    }

    #[test]
    fn validate_scroll_requires_direction_and_amount() {
        let r = ComputerActionRequest {
            coordinate: Some([0, 0]),
            ..req(ComputerAction::Scroll)
        };
        let result = validate_request(&r);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("scroll_direction"));
    }

    #[test]
    fn validate_scroll_requires_coordinate() {
        let r = ComputerActionRequest {
            scroll_direction: Some(ScrollDirection::Down),
            scroll_amount: Some(3),
            ..req(ComputerAction::Scroll)
        };
        let result = validate_request(&r);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("coordinate"));
    }

    #[test]
    fn validate_drag_requires_both_coordinates() {
        let r = ComputerActionRequest {
            coordinate: Some([100, 100]),
            ..req(ComputerAction::LeftClickDrag)
        };
        let result = validate_request(&r);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("start_coordinate"));
    }

    #[test]
    fn validate_screenshot_needs_nothing() {
        assert!(validate_request(&req(ComputerAction::Screenshot)).is_ok());
    }

    #[test]
    fn xdotool_args_left_click() {
        let r = ComputerActionRequest {
            coordinate: Some([100, 200]),
            ..req(ComputerAction::LeftClick)
        };
        let args = build_xdotool_args(&r);
        assert_eq!(
            args,
            vec!["mousemove", "--sync", "100", "200", "click", "1"]
        );
    }

    #[test]
    fn xdotool_args_right_click() {
        let r = ComputerActionRequest {
            coordinate: Some([50, 75]),
            ..req(ComputerAction::RightClick)
        };
        let args = build_xdotool_args(&r);
        assert_eq!(args, vec!["mousemove", "--sync", "50", "75", "click", "3"]);
    }

    #[test]
    fn xdotool_args_double_click() {
        let r = ComputerActionRequest {
            coordinate: Some([250, 350]),
            ..req(ComputerAction::DoubleClick)
        };
        let args = build_xdotool_args(&r);
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
        let r = ComputerActionRequest {
            coordinate: Some([10, 20]),
            ..req(ComputerAction::TripleClick)
        };
        let args = build_xdotool_args(&r);
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
        let r = ComputerActionRequest {
            coordinate: Some([500, 600]),
            ..req(ComputerAction::MouseMove)
        };
        let args = build_xdotool_args(&r);
        assert_eq!(args, vec!["mousemove", "--sync", "500", "600"]);
    }

    #[test]
    fn xdotool_args_left_click_drag() {
        let r = ComputerActionRequest {
            coordinate: Some([300, 300]),
            start_coordinate: Some([100, 100]),
            ..req(ComputerAction::LeftClickDrag)
        };
        let args = build_xdotool_args(&r);
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
        let r = ComputerActionRequest {
            text: Some("Return".to_string()),
            ..req(ComputerAction::Key)
        };
        let args = build_xdotool_args(&r);
        assert_eq!(args, vec!["key", "--", "Return"]);
    }

    #[test]
    fn xdotool_args_type() {
        let r = ComputerActionRequest {
            text: Some("hello world".to_string()),
            ..req(ComputerAction::Type)
        };
        let args = build_xdotool_args(&r);
        assert_eq!(args, vec!["type", "--clearmodifiers", "--", "hello world"]);
    }

    #[test]
    fn xdotool_args_hold_key() {
        let r = ComputerActionRequest {
            text: Some("shift".to_string()),
            duration: Some(500),
            ..req(ComputerAction::HoldKey)
        };
        let args = build_xdotool_args(&r);
        assert_eq!(
            args,
            vec!["keydown", "shift", "sleep", "0.5", "keyup", "shift"]
        );
    }

    #[test]
    fn xdotool_args_scroll_down() {
        let r = ComputerActionRequest {
            coordinate: Some([400, 300]),
            scroll_direction: Some(ScrollDirection::Down),
            scroll_amount: Some(3),
            ..req(ComputerAction::Scroll)
        };
        let args = build_xdotool_args(&r);
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
        let r = ComputerActionRequest {
            coordinate: Some([0, 0]),
            scroll_direction: Some(ScrollDirection::Up),
            scroll_amount: Some(5),
            ..req(ComputerAction::Scroll)
        };
        let args = build_xdotool_args(&r);
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
        let r = ComputerActionRequest {
            coordinate: Some([150, 250]),
            ..req(ComputerAction::LeftMouseDown)
        };
        let args = build_xdotool_args(&r);
        assert_eq!(
            args,
            vec!["mousemove", "--sync", "150", "250", "mousedown", "1"]
        );
    }

    #[test]
    fn xdotool_args_left_mouse_up() {
        let r = ComputerActionRequest {
            coordinate: Some([150, 250]),
            ..req(ComputerAction::LeftMouseUp)
        };
        let args = build_xdotool_args(&r);
        assert_eq!(
            args,
            vec!["mousemove", "--sync", "150", "250", "mouseup", "1"]
        );
    }

    #[test]
    fn screenshot_args_full_screen() {
        let args = build_screenshot_args(&req(ComputerAction::Screenshot));
        assert_eq!(args, vec!["-o", "/dev/stdout"]);
    }

    #[test]
    fn screenshot_args_with_region() {
        let r = ComputerActionRequest {
            region: Some([10, 20, 100, 200]),
            ..req(ComputerAction::Screenshot)
        };
        let args = build_screenshot_args(&r);
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
