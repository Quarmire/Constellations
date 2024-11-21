use axum::http::StatusCode;

pub async fn list_services() -> Result<String, StatusCode> {
    Ok("Service List: test123".to_string())
}