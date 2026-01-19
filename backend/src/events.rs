use serde::{Deserialize, Serialize};
use crate::game::{GameState, Symbol};

// ============================================
// CLIENT → SERVER EVENTS
// ============================================

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ClientEvent {
    #[serde(rename = "join_room")]
    JoinRoom {
        room_id: String,
        user_id: String,
        username: String,
    },

    #[serde(rename = "make_move")]
    MakeMove {
        room_id: String,
        row: usize,
        col: usize,
    },

    #[serde(rename = "leave_room")]
    LeaveRoom {
        room_id: String
    }
}

// ============================================
// SERVER → CLIENT EVENTS
// ============================================

#[derive(Debug, Serialize)]
pub struct PlayerInfo {
    pub username: String,
    pub symbol: Symbol,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum ServerEvent {
    #[serde(rename = "room_joined")]
    RoomJoined {
        room_id: String,
        your_symbol: Symbol,
        game_state: GameState,
        players: Vec<PlayerInfo>,
    },

    #[serde(rename = "player_joined")]
    PlayerJoined {
        username: String,
        symbol: Symbol,
    },

    #[serde(rename = "game_started")]
    GameStarted {
        game_state: GameState,
    },

    #[serde(rename = "move_made")]
    MoveMade {
        username: String,
        row: usize,
        col: usize,
        symbol: Symbol,
        game_state: GameState,
    },

    #[serde(rename = "game_over")]
    GameOver {
        winner: Option<String>, // username or None for draw
        game_state: GameState,
    },

    #[serde(rename = "player_left")]
    PlayerLeft {
        username: String,
    },

    #[serde(rename = "chat_message")]
    ChatMessage {
        username: String,
        message: String,
    },

    #[serde(rename = "error")]
    Error {
        message: String,
    },
}

impl ServerEvent {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}