use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;
use sqlx::{types::time::OffsetDateTime};
use crate::Store;

#[derive(Serialize, Deserialize)]
pub struct CreateUserResponse {
    pub user_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub email:String,
    pub username: String,
    pub password: String
}

#[derive(Serialize, Deserialize)]
pub struct UserSigninRequest {
    pub email:String,
    pub password: String
}

#[derive(Serialize, Deserialize)]
pub struct GetUserRequest {
    pub email: String,
}

#[derive(Serialize, Deserialize)]
pub struct GetUserResponse {
    pub user: User,
}

#[derive(Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub username: String,
    pub password: String,
    pub email:String
}

#[derive(Serialize, Deserialize)]
pub struct UserStats {
    pub user_id: Uuid,
    pub total_games: Option<i32>,
    pub wins: Option<i32>,
    pub losses: Option<i32>,
    pub draws: Option<i32>,
    pub current_streak: Option<i32>,
    pub longest_streak: Option<i32>,
    pub updated_at: Option<OffsetDateTime>,
}

#[derive(Serialize, Deserialize)]
pub enum UpdateUserStatsType {
    Win,
    Loss,
    Draw
}

#[derive(Serialize, Deserialize)]
pub struct UpdateUserStatsRequest {
    pub user_id: Uuid,
    pub update_type: UpdateUserStatsType,
}

#[derive(Serialize, Deserialize)]
pub struct UpdateUserStatsResponse {
    pub user_stat: UserStats,
}

#[derive(Serialize, Deserialize)]
pub struct GetUserStatsRequest {
    pub user_id:Uuid
}

impl Store {
    pub async fn get_user_by_id(&self, id: String) -> Result<GetUserResponse> {
        let user = sqlx::query_as!(User, "SELECT id, username, password,email FROM users WHERE id = $1", Uuid::parse_str(&id)?)
            .fetch_one(&self.pool)
            .await?;
  
        Ok(GetUserResponse {
            user: user,
        })
    }
    

    pub async fn create_user(&self, request:CreateUserRequest) -> Result<CreateUserResponse> {
        let user =  sqlx::query_as!(User,"INSERT INTO users (username, password, email) VALUES ($1, $2, $3) RETURNING id, username, password, email",request.username,request.password,request.email)
            .fetch_one(&self.pool)
            .await?;

        Ok(CreateUserResponse {
            user_id:user.id
        })
    }

    pub async fn get_user(&self, request: GetUserRequest) -> Result<GetUserResponse> {
        let user = sqlx::query_as!(User, "SELECT id, username, password,email FROM users WHERE email = $1", request.email)
            .fetch_one(&self.pool)
            .await?;

        Ok(GetUserResponse {
            user: user,
        })
    }

    pub async fn update_user_stats(&self, request:UpdateUserStatsRequest) -> Result<UpdateUserStatsResponse> {
        let user_stat;
        match request.update_type {
            UpdateUserStatsType::Win => {
                user_stat = sqlx::query_as!(UserStats,"UPDATE user_stats SET total_games = total_games + 1, wins = wins + 1 WHERE user_id = $1 RETURNING user_id,total_games,wins,losses,draws,current_streak,longest_streak,updated_at",request.user_id)
                .fetch_one(&self.pool)
                .await?;
            }
            UpdateUserStatsType::Loss => {
                user_stat = sqlx::query_as!(UserStats,"UPDATE user_stats SET total_games = total_games + 1, losses = losses + 1 WHERE user_id = $1 RETURNING user_id,total_games,wins,losses,draws,current_streak,longest_streak,updated_at",request.user_id)
                .fetch_one(&self.pool)
                .await?;
            }
            UpdateUserStatsType::Draw => {
                user_stat = sqlx::query_as!(UserStats,"UPDATE user_stats SET total_games = total_games + 1, draws = draws + 1 WHERE user_id = $1 RETURNING user_id,total_games,wins,losses,draws,current_streak,longest_streak,updated_at",request.user_id)
                .fetch_one(&self.pool)
                .await?;
            }
        }

        Ok(UpdateUserStatsResponse {
            user_stat: user_stat,
        })
        
    }

    
    pub async fn get_user_stats(&self, request:GetUserStatsRequest ) -> Result<UserStats> {
        let user_stat = sqlx::query_as!(UserStats,"SELECT user_id,total_games,wins,losses,draws,current_streak,longest_streak,updated_at FROM user_stats WHERE user_id = $1",request.user_id)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(stats) = user_stat {
            Ok(stats)
        } else {
            let initial_stats = sqlx::query_as!(UserStats,"INSERT INTO user_stats (user_id, total_games, wins, losses, draws, current_streak, longest_streak) VALUES ($1, 0, 0, 0, 0, 0, 0) RETURNING user_id,total_games,wins,losses,draws,current_streak,longest_streak,updated_at",request.user_id)
                .fetch_one(&self.pool)
                .await?;
            Ok(initial_stats)
        }
    }
}