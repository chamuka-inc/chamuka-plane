#![allow(clippy::unwrap_used, clippy::expect_used)]

use common::ext::agentd::{
    ComputerAction, ComputerActionRequest, ComputerActionResponse, ExecRequest, ExecSyncResponse,
};
use common::test_helpers::{load_fixture, load_fixture_as};
use reqwest::StatusCode;

#[tokio::test]
async fn contract_exec_minimal_fixture() {
    let addr = agentd::serve_on_ephemeral().await;
    let url = format!("http://{addr}/v1/exec");

    let req: ExecRequest = load_fixture_as("agentd/exec-request.minimal.json");
    assert_eq!(req.cmd, "echo");
    assert_eq!(req.args, vec!["hello"]);

    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .json(&req)
        .send()
        .await
        .expect("request failed");

    assert_eq!(resp.status(), StatusCode::OK);
    let body: ExecSyncResponse = resp.json().await.expect("json parse failed");
    assert_eq!(body.exit_code, 0);
    assert_eq!(body.stdout.trim(), "hello");
}

#[tokio::test]
async fn contract_exec_full_fixture_deserializes() {
    let req: ExecRequest = load_fixture_as("agentd/exec-request.full.json");
    assert_eq!(req.cmd, "python3");
    assert!(req.pty);
    assert!(req.pty_size.is_some());
    assert_eq!(req.timeout_ms, Some(30000));
    assert_eq!(req.user, Some("sandbox".to_string()));
}

#[tokio::test]
async fn contract_exec_response_fixture_deserializes() {
    let resp: ExecSyncResponse = load_fixture_as("agentd/exec-sync-response.json");
    assert_eq!(resp.cmd_id, "cmd-abc123");
    assert_eq!(resp.exit_code, 0);
    assert!(!resp.stdout.is_empty());
}

#[tokio::test]
async fn contract_computer_screenshot_fixture() {
    let fixture = load_fixture("agentd/computer-screenshot.json");
    let req: ComputerActionRequest =
        serde_json::from_value(fixture["request"].clone()).expect("parse request");
    assert_eq!(req.action, ComputerAction::Screenshot);
    assert!(req.coordinate.is_none());

    let resp: ComputerActionResponse =
        serde_json::from_value(fixture["response"].clone()).expect("parse response");
    match &resp.output {
        common::ext::agentd::ComputerOutput::Image { media_type, .. } => {
            assert_eq!(media_type, "image/png");
        }
        other => panic!("expected Image output, got {other:?}"),
    }
}

#[tokio::test]
async fn contract_computer_cursor_position_fixture() {
    let fixture = load_fixture("agentd/computer-cursor-position.json");
    let req: ComputerActionRequest =
        serde_json::from_value(fixture["request"].clone()).expect("parse request");
    assert_eq!(req.action, ComputerAction::CursorPosition);

    let resp: ComputerActionResponse =
        serde_json::from_value(fixture["response"].clone()).expect("parse response");
    match &resp.output {
        common::ext::agentd::ComputerOutput::Text { text } => {
            assert!(text.contains(','));
        }
        other => panic!("expected Text output, got {other:?}"),
    }
}

#[tokio::test]
async fn contract_computer_left_click_fixture() {
    let fixture = load_fixture("agentd/computer-left-click.json");
    let req: ComputerActionRequest =
        serde_json::from_value(fixture["request"].clone()).expect("parse request");
    assert_eq!(req.action, ComputerAction::LeftClick);
    assert_eq!(req.coordinate, Some([100, 200]));

    let resp: ComputerActionResponse =
        serde_json::from_value(fixture["response"].clone()).expect("parse response");
    assert!(matches!(
        resp.output,
        common::ext::agentd::ComputerOutput::Ok
    ));
}

#[tokio::test]
async fn contract_computer_type_fixture() {
    let fixture = load_fixture("agentd/computer-type.json");
    let req: ComputerActionRequest =
        serde_json::from_value(fixture["request"].clone()).expect("parse request");
    assert_eq!(req.action, ComputerAction::Type);
    assert_eq!(req.text.as_deref(), Some("hello world"));

    let resp: ComputerActionResponse =
        serde_json::from_value(fixture["response"].clone()).expect("parse response");
    assert!(matches!(
        resp.output,
        common::ext::agentd::ComputerOutput::Ok
    ));
}

#[tokio::test]
async fn contract_computer_key_fixture() {
    let fixture = load_fixture("agentd/computer-key.json");
    let req: ComputerActionRequest =
        serde_json::from_value(fixture["request"].clone()).expect("parse request");
    assert_eq!(req.action, ComputerAction::Key);
    assert_eq!(req.text.as_deref(), Some("Return"));
}

#[tokio::test]
async fn contract_computer_scroll_fixture() {
    let fixture = load_fixture("agentd/computer-scroll.json");
    let req: ComputerActionRequest =
        serde_json::from_value(fixture["request"].clone()).expect("parse request");
    assert_eq!(req.action, ComputerAction::Scroll);
    assert_eq!(req.coordinate, Some([400, 300]));
    assert_eq!(
        req.scroll_direction,
        Some(common::ext::agentd::ScrollDirection::Down)
    );
    assert_eq!(req.scroll_amount, Some(3));
}

#[tokio::test]
async fn contract_computer_mouse_move_fixture() {
    let fixture = load_fixture("agentd/computer-mouse-move.json");
    let req: ComputerActionRequest =
        serde_json::from_value(fixture["request"].clone()).expect("parse request");
    assert_eq!(req.action, ComputerAction::MouseMove);
    assert_eq!(req.coordinate, Some([500, 600]));
}

#[tokio::test]
async fn contract_computer_drag_fixture() {
    let fixture = load_fixture("agentd/computer-left-click-drag.json");
    let req: ComputerActionRequest =
        serde_json::from_value(fixture["request"].clone()).expect("parse request");
    assert_eq!(req.action, ComputerAction::LeftClickDrag);
    assert_eq!(req.start_coordinate, Some([100, 100]));
    assert_eq!(req.coordinate, Some([300, 300]));
}

#[tokio::test]
async fn contract_computer_double_click_fixture() {
    let fixture = load_fixture("agentd/computer-double-click.json");
    let req: ComputerActionRequest =
        serde_json::from_value(fixture["request"].clone()).expect("parse request");
    assert_eq!(req.action, ComputerAction::DoubleClick);
    assert_eq!(req.coordinate, Some([250, 350]));
}

#[tokio::test]
async fn contract_computer_hold_key_fixture() {
    let fixture = load_fixture("agentd/computer-hold-key.json");
    let req: ComputerActionRequest =
        serde_json::from_value(fixture["request"].clone()).expect("parse request");
    assert_eq!(req.action, ComputerAction::HoldKey);
    assert_eq!(req.text.as_deref(), Some("shift"));
    assert_eq!(req.duration, Some(500));
}

#[tokio::test]
async fn contract_computer_all_fixtures_validate() {
    let fixtures = [
        "agentd/computer-screenshot.json",
        "agentd/computer-cursor-position.json",
        "agentd/computer-left-click.json",
        "agentd/computer-type.json",
        "agentd/computer-key.json",
        "agentd/computer-scroll.json",
        "agentd/computer-mouse-move.json",
        "agentd/computer-left-click-drag.json",
        "agentd/computer-double-click.json",
        "agentd/computer-hold-key.json",
    ];

    for fixture_name in &fixtures {
        let fixture = load_fixture(fixture_name);
        let req: ComputerActionRequest = serde_json::from_value(fixture["request"].clone())
            .unwrap_or_else(|e| panic!("Failed to parse request in {fixture_name}: {e}"));
        agentd::computer::validate_request(&req)
            .unwrap_or_else(|e| panic!("Validation failed for {fixture_name}: {e}"));
    }
}
