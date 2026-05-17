#![allow(clippy::unwrap_used, clippy::expect_used)]

use common::ext::agentd::{ComputerAction, ComputerActionRequest};
use common::test_helpers::load_fixture;
use reqwest::StatusCode;

#[tokio::test]
async fn computer_endpoint_responds() {
    let addr = agentd::serve_on_ephemeral().await;
    let url = format!("http://{addr}/v1/computer");
    let client = reqwest::Client::new();
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

    let resp = client
        .post(&url)
        .json(&req)
        .send()
        .await
        .expect("request failed");

    #[cfg(not(feature = "desktop"))]
    assert_eq!(resp.status(), StatusCode::NOT_IMPLEMENTED);

    // With desktop feature but no X display, execute() fails → 500.
    // In a real X11 environment it would return 200.
    #[cfg(feature = "desktop")]
    assert!(
        resp.status() == StatusCode::OK || resp.status() == StatusCode::INTERNAL_SERVER_ERROR,
        "expected 200 or 500, got {}",
        resp.status()
    );
}

#[tokio::test]
async fn computer_endpoint_rejects_missing_coordinate() {
    let addr = agentd::serve_on_ephemeral().await;
    let url = format!("http://{addr}/v1/computer");
    let client = reqwest::Client::new();

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

    let resp = client
        .post(&url)
        .json(&req)
        .send()
        .await
        .expect("request failed");

    #[cfg(not(feature = "desktop"))]
    assert_eq!(resp.status(), StatusCode::NOT_IMPLEMENTED);

    #[cfg(feature = "desktop")]
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

#[test]
fn xdotool_args_match_left_click_fixture() {
    let fixture = load_fixture("agentd/computer-left-click.json");
    let req: ComputerActionRequest =
        serde_json::from_value(fixture["request"].clone()).expect("parse fixture");
    let args = agentd::computer::build_xdotool_args(&req);
    assert_eq!(
        args,
        vec!["mousemove", "--sync", "100", "200", "click", "1"]
    );
}

#[test]
fn xdotool_args_match_scroll_fixture() {
    let fixture = load_fixture("agentd/computer-scroll.json");
    let req: ComputerActionRequest =
        serde_json::from_value(fixture["request"].clone()).expect("parse fixture");
    let args = agentd::computer::build_xdotool_args(&req);
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
fn xdotool_args_match_type_fixture() {
    let fixture = load_fixture("agentd/computer-type.json");
    let req: ComputerActionRequest =
        serde_json::from_value(fixture["request"].clone()).expect("parse fixture");
    let args = agentd::computer::build_xdotool_args(&req);
    assert_eq!(args, vec!["type", "--clearmodifiers", "--", "hello world"]);
}

#[test]
fn xdotool_args_match_key_fixture() {
    let fixture = load_fixture("agentd/computer-key.json");
    let req: ComputerActionRequest =
        serde_json::from_value(fixture["request"].clone()).expect("parse fixture");
    let args = agentd::computer::build_xdotool_args(&req);
    assert_eq!(args, vec!["key", "--", "Return"]);
}

#[test]
fn xdotool_args_match_mouse_move_fixture() {
    let fixture = load_fixture("agentd/computer-mouse-move.json");
    let req: ComputerActionRequest =
        serde_json::from_value(fixture["request"].clone()).expect("parse fixture");
    let args = agentd::computer::build_xdotool_args(&req);
    assert_eq!(args, vec!["mousemove", "--sync", "500", "600"]);
}

#[test]
fn xdotool_args_match_drag_fixture() {
    let fixture = load_fixture("agentd/computer-left-click-drag.json");
    let req: ComputerActionRequest =
        serde_json::from_value(fixture["request"].clone()).expect("parse fixture");
    let args = agentd::computer::build_xdotool_args(&req);
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
fn xdotool_args_match_double_click_fixture() {
    let fixture = load_fixture("agentd/computer-double-click.json");
    let req: ComputerActionRequest =
        serde_json::from_value(fixture["request"].clone()).expect("parse fixture");
    let args = agentd::computer::build_xdotool_args(&req);
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
fn xdotool_args_match_hold_key_fixture() {
    let fixture = load_fixture("agentd/computer-hold-key.json");
    let req: ComputerActionRequest =
        serde_json::from_value(fixture["request"].clone()).expect("parse fixture");
    let args = agentd::computer::build_xdotool_args(&req);
    assert_eq!(
        args,
        vec!["keydown", "shift", "sleep", "0.5", "keyup", "shift"]
    );
}

#[test]
fn screenshot_args_match_full_screen() {
    let fixture = load_fixture("agentd/computer-screenshot.json");
    let req: ComputerActionRequest =
        serde_json::from_value(fixture["request"].clone()).expect("parse fixture");
    let args = agentd::computer::build_screenshot_args(&req);
    assert_eq!(args, vec!["-o", "/dev/stdout"]);
}
