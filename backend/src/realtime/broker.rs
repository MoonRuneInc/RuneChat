use dashmap::DashMap;
use futures_util::StreamExt;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

pub async fn run(
    redis_url: String,
    db: PgPool,
    ws_senders: Arc<DashMap<Uuid, tokio::sync::mpsc::UnboundedSender<String>>>,
) {
    loop {
        match try_run(&redis_url, &db, &ws_senders).await {
            Ok(()) => {
                tracing::info!("broker loop exited cleanly");
                break;
            }
            Err(e) => {
                tracing::error!("broker error: {e} — reconnecting in 3s");
                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
            }
        }
    }
}

async fn try_run(
    redis_url: &str,
    db: &PgPool,
    ws_senders: &Arc<DashMap<Uuid, tokio::sync::mpsc::UnboundedSender<String>>>,
) -> anyhow::Result<()> {
    let client = redis::Client::open(redis_url)?;
    let conn = client.get_async_connection().await?;

    let mut pubsub = conn.into_pubsub();
    pubsub.psubscribe("channel:*").await?;

    tracing::info!("broker subscribed to channel:* on Redis");

    let mut stream = pubsub.into_on_message();

    while let Some(msg) = stream.next().await {
        let payload: String = match msg.get_payload() {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("broker: failed to decode payload: {e}");
                continue;
            }
        };

        let value: serde_json::Value = match serde_json::from_str(&payload) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("broker: invalid JSON payload: {e}");
                continue;
            }
        };

        let channel_id_str = match value.get("channel_id").and_then(|v| v.as_str()) {
            Some(s) => s.to_string(),
            None => {
                tracing::warn!("broker: no channel_id in payload");
                continue;
            }
        };

        let channel_id = match Uuid::parse_str(&channel_id_str) {
            Ok(id) => id,
            Err(_) => continue,
        };

        // Look up which users are members of this channel's server
        let member_ids = match fetch_channel_members(db, channel_id).await {
            Ok(ids) => ids,
            Err(e) => {
                tracing::warn!("broker: DB error fetching members for channel {channel_id}: {e}");
                continue;
            }
        };

        // Fan out to connected users
        let mut dead_senders = Vec::new();
        for user_id in &member_ids {
            if let Some(sender) = ws_senders.get(user_id) {
                if sender.send(payload.clone()).is_err() {
                    dead_senders.push(*user_id);
                }
            }
        }

        // Clean up disconnected senders
        for user_id in dead_senders {
            ws_senders.remove(&user_id);
        }
    }

    Ok(())
}

async fn fetch_channel_members(db: &PgPool, channel_id: Uuid) -> Result<Vec<Uuid>, sqlx::Error> {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT sm.user_id
        FROM server_members sm
        JOIN channels c ON c.server_id = sm.server_id
        JOIN users u ON u.id = sm.user_id
        WHERE c.id = $1 AND u.account_status != 'compromised'
        "#,
    )
    .bind(channel_id)
    .fetch_all(db)
    .await
}
