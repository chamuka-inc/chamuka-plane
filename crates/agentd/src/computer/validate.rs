use super::ComputerError;
use common::ext::agentd::{ComputerAction, ComputerActionRequest};

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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use common::ext::agentd::ScrollDirection;

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
}
