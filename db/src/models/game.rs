use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;
use sqlx::{types::time::OffsetDateTime,Type};
use crate::Store;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "game_status", rename_all = "snake_case")]
pub enum GameStatus {
    InProgress,
    Completed,
}

#[derive(Serialize, Deserialize)]
pub struct Game {
    pub id: Uuid,
    pub room_id: Option<Uuid>,
    pub player1_id: Option<Uuid>,
    pub player2_id: Option<Uuid>,
    pub winner_id: Option<Uuid>,
    pub status: Option<GameStatus>,
    pub total_moves: Option<i32>,
    pub duration_seconds: Option<i32>,
    pub completed_at: Option<OffsetDateTime>,
    pub created_at: Option<OffsetDateTime>,
}

#[derive(Serialize, Deserialize)]
pub struct GetGameHistoryRequest {
    pub user_id:Uuid
}

impl Store {
    pub async fn get_game_history(&self) -> Result<Vec<Game>> {
        let games = sqlx::query_as!(Game, r#"SELECT id,room_id,player1_id,player2_id,winner_id,status as "status: GameStatus",total_moves,duration_seconds,completed_at,created_at FROM games"#)
            .fetch_all(&self.pool)
            .await?;

        Ok(games)
    }
}