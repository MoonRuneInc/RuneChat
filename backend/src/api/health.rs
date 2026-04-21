use axum::{http::StatusCode, Json};
use serde_json::{json, Value};

pub async fn health_check() -> (StatusCode, Json<Value>) {
    (StatusCode::OK, Json(json!({ "status": "ok" })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::Router;
    use axum_test::TestServer;

    fn health_router() -> Router {
        Router::new().route("/health", axum::routing::get(health_check))
    }

    #[tokio::test]
    async fn health_returns_200_with_ok_status() {
        let server = TestServer::new(health_router()).unwrap();
        let resp = server.get("/health").await;
        resp.assert_status_ok();
        let body: serde_json::Value = resp.json();
        assert_eq!(body["status"], "ok");
    }
}
