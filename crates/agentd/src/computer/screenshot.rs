use common::ext::agentd::ComputerActionRequest;

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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use common::ext::agentd::ComputerAction;

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
}
