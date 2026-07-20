use sqlx::PgPool;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/fiscal_core".to_string());
    let pool = PgPool::connect(&url).await?;

    println!("Running migrations...");

    // Use raw_sql for multiple statements (sqlx 0.8+)
    // Or split into separate queries if raw_sql not available
    sqlx::raw_sql(
        r#"
        CREATE EXTENSION IF NOT EXISTS "pgcrypto";

        CREATE TABLE IF NOT EXISTS events (
            event_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            aggregate_type TEXT NOT NULL,
            aggregate_id UUID NOT NULL,
            version BIGINT NOT NULL,
            event_type TEXT NOT NULL,
            payload JSONB NOT NULL,
            metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
            tenant_id TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            prev_hash TEXT,
            hash TEXT NOT NULL,
            UNIQUE(aggregate_id, version)
        );

        CREATE INDEX IF NOT EXISTS idx_events_aggregate ON events (aggregate_id, version);
        CREATE INDEX IF NOT EXISTS idx_events_tenant_type ON events (tenant_id, aggregate_type);

        CREATE TABLE IF NOT EXISTS snapshots (
            aggregate_id UUID PRIMARY KEY,
            aggregate_type TEXT NOT NULL,
            version BIGINT NOT NULL,
            state JSONB NOT NULL,
            created_at TIMESTAMPTZ DEFAULT NOW()
        );

        CREATE TABLE IF NOT EXISTS read_employee_balances (
            id TEXT PRIMARY KEY,
            tenant_id TEXT NOT NULL,
            accrued_net DECIMAL(12,2) DEFAULT 0,
            advance_balance DECIMAL(12,2) DEFAULT 0,
            available_advance DECIMAL(12,2) DEFAULT 0,
            updated_at TIMESTAMPTZ DEFAULT NOW()
        );

        CREATE TABLE IF NOT EXISTS read_sales (
            id TEXT PRIMARY KEY,
            tenant_id TEXT NOT NULL,
            e_ncf TEXT,
            tipo_ecf INT,
            total DECIMAL(12,2),
            itbis_total DECIMAL(12,2),
            status_dgii TEXT,
            track_id TEXT,
            qr_url TEXT,
            created_at TIMESTAMPTZ DEFAULT NOW()
        );
        "#
    )
    .execute(&pool)
    .await?;

    println!("Migrations OK - EventStore + read models created");
    Ok(())
}
