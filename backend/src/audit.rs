use sqlx::PgPool;
use uuid::Uuid;

pub async fn log(
    db: &PgPool,
    event_type: &str,
    user_id: Option<Uuid>,
    ip: Option<&str>,
    details: Option<serde_json::Value>,
) {
    let result = sqlx::query(
        "INSERT INTO audit_log (event_type, user_id, ip, details) VALUES ($1, $2, $3, $4)",
    )
    .bind(event_type)
    .bind(user_id)
    .bind(ip)
    .bind(details)
    .execute(db)
    .await;

    if let Err(e) = result {
        tracing::warn!("audit log insert failed for event '{event_type}': {e}");
    }
}
