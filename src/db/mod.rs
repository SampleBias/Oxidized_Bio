use sqlx::postgres::{PgPool, PgPoolOptions};
use crate::config::DatabaseConfig;
use anyhow::Result;

pub use operations::*;
pub use pool::*;

pub mod pool;
pub mod operations;

pub async fn create_pool(config: &DatabaseConfig) -> Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .connect(&config.url)
        .await?;

    // Test connection
    sqlx::query("SELECT 1")
        .fetch_one(&pool)
        .await?;

    Ok(pool)
}
