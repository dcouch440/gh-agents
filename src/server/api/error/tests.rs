#[cfg(test)]
mod tests {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;

    use super::AppError;

    #[test]
    fn bad_request_returns_400() {
        let resp = AppError::bad_request("invalid input").into_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn unauthorized_returns_401() {
        let resp = AppError::Unauthorized("bad token".into()).into_response();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn forbidden_returns_403() {
        let resp = AppError::Forbidden("access denied".into()).into_response();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn not_found_returns_404() {
        let resp = AppError::not_found("Agent").into_response();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn conflict_returns_409() {
        let resp = AppError::Conflict("duplicate".into()).into_response();
        assert_eq!(resp.status(), StatusCode::CONFLICT);
    }

    #[test]
    fn service_unavailable_returns_503() {
        let resp = AppError::ServiceUnavailable("LLM down".into()).into_response();
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[test]
    fn internal_returns_500() {
        let resp = AppError::Internal("db failed".into()).into_response();
        assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn from_anyhow_produces_internal() {
        let err: AppError = anyhow::anyhow!("something broke").into();
        assert!(matches!(err, AppError::Internal(_)));
        assert_eq!(err.to_string(), "something broke");
    }

    #[test]
    fn not_found_helper_formats_message() {
        let err = AppError::not_found("Tool");
        assert_eq!(err.to_string(), "Tool not found");
    }

    #[test]
    fn display_matches_message() {
        let err = AppError::BadRequest("bad input".into());
        assert_eq!(format!("{err}"), "bad input");
    }
}
