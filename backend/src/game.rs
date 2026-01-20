use serde::{Serialize,Deserialize};
use std::{collections::HashSet, sync::Arc};
use tokio::sync::RwLock;
use std::collections::HashMap;
use tokio::sync::mpsc::Sender;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Symbol {
    X,
    O,
}

pub struct User {
    pub tx: Sender<String>,
}


pub struct Room {
    pub room_id: String,
    pub room_name: String,
    pub players: Vec<Player>,
    pub spectators: Vec<String>, // user_ids
    pub game_state: GameState,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GameStatus {
    Waiting,      // Waiting for second player
    InProgress,   // Game is active
    Completed,    // Game finished
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GameResult {
    Win(String),  // winner's user_id
    Draw,
}

#[derive(Clone)]
pub struct Player {
    pub user_id: String,
    pub username: String,
    pub symbol: Symbol,
    pub connected: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct GameState {
    pub board: [[Option<Symbol>; 3]; 3],
    pub current_turn: Symbol,
    pub status: GameStatus,
    pub result: Option<GameResult>,
    pub move_count: u8,
}

pub type Rooms = Arc<RwLock<HashMap<String, Room>>>;



pub struct WaitingPlayer {
    pub user_id:String,
    pub username:String
}

pub struct RoomManager {
    pub clients: HashMap<String, User>,
    pub subscriptions: HashMap<String, HashSet<String>>,
    pub rooms: HashMap<String, Room>,
    pub waiting_queue: Vec<WaitingPlayer>
}


impl RoomManager {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
            subscriptions: HashMap::new(),
            rooms: HashMap::new(),
            waiting_queue: Vec::new()
        }
    }

    pub async fn broadcast(&self,room_id:&str,message:String) {
        if let Some(subscriptions) = self.subscriptions.get(room_id) {
            for user_id in subscriptions {
                if let Some(user) = self.clients.get(user_id) {
                    let _ = user.tx.send(message.clone()).await;
                }
            }
        }
    }

    pub async fn broadcast_to_user(&self,message:String,user_id:String) {
        if let Some(user) = self.clients.get(&user_id) {
            let _ = user.tx.send(message.clone()).await;
        }
    }

    // pub async fn find_match(&self,user_id:String,username:String){
    //     if(self.waiting_queue.len() == 0){
    //         self.waiting_queue.a;
    //     }
    // }
}
pub struct AppState {
    pub room_manager: Arc<RwLock<RoomManager>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            room_manager: Arc::new(RwLock::new(RoomManager::new())),
        }
    }
}


impl GameState {
    pub fn new() -> Self {
        Self {
            board: [[None; 3]; 3],
            current_turn: Symbol::X,
            status: GameStatus::Waiting,
            result: None,
            move_count: 0,
        }
    }

    pub fn make_move(&mut self, row: usize, col: usize, symbol: Symbol,user_id:String) -> Result<bool, String> {
        // Validate move
        if self.status != GameStatus::InProgress {
            return Err("Game not in progress".to_string());
        }

        if self.current_turn != symbol {
            return Err("Not your turn".to_string());
        }

        if row >= 3 || col >= 3 {
            return Err("Invalid position".to_string());
        }

        if self.board[row][col].is_some() {
            return Err("Position already occupied".to_string());
        }

        // Make the move
        self.board[row][col] = Some(symbol);
        self.move_count += 1;

        // Check for winner
        if self.check_winner(symbol) {
            self.status = GameStatus::Completed;
            self.result = Some(GameResult::Win(user_id));
            return Ok(true); 
        }

        // Check for draw (all 9 squares filled)
        if self.move_count == 9 {
            self.status = GameStatus::Completed;
            self.result = Some(GameResult::Draw);
            return Ok(true); 
        }

        // Switch turn
        self.current_turn = match symbol {
            Symbol::X => Symbol::O,
            Symbol::O => Symbol::X,
        };

        Ok(false) // Game continues
    }


    fn check_winner(&self, symbol: Symbol) -> bool {
        let s = Some(symbol);

        // Check rows
        for row in 0..3 {
            if self.board[row][0] == s && self.board[row][1] == s && self.board[row][2] == s {
                return true;
            }
        }

        // Check columns
        for col in 0..3 {
            if self.board[0][col] == s && self.board[1][col] == s && self.board[2][col] == s {
                return true;
            }
        }

        // Check diagonals
        if self.board[0][0] == s && self.board[1][1] == s && self.board[2][2] == s {
            return true;
        }
        if self.board[0][2] == s && self.board[1][1] == s && self.board[2][0] == s {
            return true;
        }

        false
    }

}



impl Room {
    pub fn new(room_id: String, room_name: String) -> Self {
        Self {
            room_id,
            room_name,
            players: Vec::new(),
            spectators: Vec::new(),
            game_state: GameState::new(),
            created_at: chrono::Utc::now(),
        }
    }

    pub fn add_player(&mut self, player: Player) -> Result<(), String> {
        if self.players.iter().any(|p| p.user_id == player.user_id) {
            let existing_player = self.get_player_mut(&player.user_id).ok_or("Player not found")?;
            existing_player.connected = true;
            return Ok(());
        }

        // Assign symbol based on position
        let symbol = if self.players.is_empty() {
            Symbol::X
        } else {
            Symbol::O
        };

        let mut player = player;
        player.symbol = symbol;
        
        self.players.push(player);

        // Start game when both players joined
        if self.players.len() == 2 {
            self.game_state.status = GameStatus::InProgress;
        }

        Ok(())
    }

    pub fn remove_player(&mut self,user_id:String) -> Result<(), String>  {
        if self.players.len() <= 0 {
            return Err("Room is already empty".to_string());
        }
 
        self.players.retain(|x| x.user_id != user_id);

        if self.players.is_empty() {
            self.game_state.status = GameStatus::Completed;
            self.game_state.result = Some(GameResult::Draw);
            return Ok(());
        }
    
        Ok(())
    }


    pub fn get_player(&self, user_id: &str) -> Option<&Player> {
        self.players.iter().find(|p| p.user_id == user_id)
    }

    pub fn get_player_mut(&mut self, user_id: &str) -> Option<&mut Player> {
        self.players.iter_mut().find(|p| p.user_id == user_id)
    }
}