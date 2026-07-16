//! EventStore - Postgres append-only, immutable, hash-chained
//! Source of truth - never UPDATE/DELETE

use sqlx::PgPool;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
pub struct Event {
    pub event_id: Uuid,
    pub aggregate_type: String,
    pub aggregate_id: Uuid,
    pub version: i64,
    pub event_type: String,
    pub payload: Value,
    pub metadata: Value,
    pub tenant_id: String, // RNC
    pub created_at: DateTime<Utc>,
    pub prev_hash: Option<String>,
    pub hash: String,
}

pub async fn create_pool() -> anyhow::Result<PgPool> {
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/fiscal_core".to_string());
    let pool = PgPool::connect(&url).await?;
    Ok(pool)
}

/// Append event with optimistic locking + hash chain
pub async fn append_event(
    pool: &PgPool,
    aggregate_type: &str,
    aggregate_id: Uuid,
    expected_version: i64, // last version, new = expected+1, 0 for new aggregate
    event_type: &str,
    payload: Value,
    metadata: Value,
    tenant_id: &str,
) -> anyhow::Result<Event> {
    // 1. Get prev_hash
    let prev: Option<(String,)> = sqlx::query_as(
        "SELECT hash FROM events WHERE aggregate_id = $1 ORDER BY version DESC LIMIT 1"
    )
    .bind(aggregate_id)
    .fetch_optional(pool)
    .await?;

    let prev_hash = prev.map(|r| r.0).unwrap_or_else(|| "0".to_string());
    
    // 2. Compute hash = SHA256(prev_hash + payload + version)
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(prev_hash.as_bytes());
    hasher.update(payload.to_string().as_bytes());
    hasher.update((expected_version + 1).to_string().as_bytes());
    let hash = format!("{:x}", hasher.finalize());

    let event = sqlx::query_as::<_, Event>(
        r#"
        INSERT INTO events (aggregate_type, aggregate_id, version, event_type, payload, metadata, tenant_id, prev_hash, hash)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING *
        "#
    )
    .bind(aggregate_type)
    .bind(aggregate_id)
    .bind(expected_version + 1)
    .bind(event_type)
    .bind(&payload)
    .bind(&metadata)
    .bind(tenant_id)
    .bind(&prev_hash)
    .bind(&hash)
    .fetch_one(pool)
    .await?;

    // 3. NOTIFY for projectors
    sqlx::query("SELECT pg_notify('events', $1)")
        .bind(serde_json::to_string(&event)?)
        .execute(pool)
        .await?;

    Ok(event)
}

pub async fn load_aggregate_events(pool: &PgPool, aggregate_id: Uuid) -> anyhow::Result<Vec<Event>> {
    let events = sqlx::query_as::<_, Event>(
        "SELECT * FROM events WHERE aggregate_id = $1 ORDER BY version ASC"
    )
    .bind(aggregate_id)
    .fetch_all(pool)
    .await?;
    Ok(events)
}
