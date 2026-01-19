use std::{env};
use anyhow::Result;
use sqlx::{PgPool, postgres::PgPoolOptions};

pub mod models;

#[derive(Clone)]
pub struct Store {
    pub pool: PgPool
}

impl Store {
    pub async fn new() -> Result<Self> {
        let db_url = env::var("DATABASE_URL")?;
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(&db_url).await?;

        Ok(Self { pool })
    }
}