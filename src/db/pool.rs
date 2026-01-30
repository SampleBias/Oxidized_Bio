use sqlx::postgres::PgPool;
use tracing::info;

pub async fn get_pool() -> Option<PgPool> {
    // Pool is typically injected via AppState
    None
}

pub async fn health_check(pool: &PgPool) -> anyhow::Result<bool> {
    let result = sqlx::query("SELECT 1")
        .fetch_one(pool)
        .await?;

    Ok(true)
}
