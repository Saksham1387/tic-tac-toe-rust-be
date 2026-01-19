use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_ws::AggregatedMessage;
use futures_util::StreamExt;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::game::{AppState, Player, Room};
use crate::events::{ClientEvent, PlayerInfo, ServerEvent};

pub async fn ws_handler(
    req: HttpRequest,
    stream: web::Payload,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, Error> {
    let (res, session, stream) = actix_ws::handle(&req, stream)?;

    let mut stream = stream
        .aggregate_continuations()
        .max_continuation_size(2_usize.pow(20));

    let session = Arc::new(RwLock::new(session));
    let rooms = app_state.rooms.clone();

    // Spawn task to handle this connection
    actix_web::rt::spawn(async move {
        let mut current_room_id: Option<String> = None;
        let mut user_id: Option<String> = None;

        while let Some(msg) = stream.next().await {
            match msg {
                Ok(AggregatedMessage::Text(text)) => {
                    // Parse incoming event
                    let event: Result<ClientEvent, _> = serde_json::from_str(&text);

                    match event {
                        Ok(ClientEvent::JoinRoom { room_id, user_id: uid, username }) => {
                            user_id = Some(uid.clone());
                            
                            let result = join_room(
                                rooms.clone(),
                                session.clone(),
                                room_id.clone(),
                                uid,
                                username,
                            ).await;

                            match result {
                                Ok(_) => {
                                    current_room_id = Some(room_id);
                                }
                                Err(e) => {
                                    send_error(session.clone(), &e).await;
                                }
                            }
                        }

                        Ok(ClientEvent::MakeMove { room_id, row, col }) => {
                            if let Some(uid) = &user_id {
                                let result = make_move(
                                    rooms.clone(),
                                    room_id,
                                    uid.clone(),
                                    row,
                                    col,
                                ).await;

                                if let Err(e) = result {
                                    send_error(session.clone(), &e).await;
                                }
                            }
                        }

                        Ok(ClientEvent::LeaveRoom { room_id }) => {
                            if let Some(uid) = &user_id {
                                let result = leave_room(
                                    rooms.clone(),
                                    room_id.clone(),
                                    uid.clone(),
                                ).await;

                                match result {
                                    Ok(_) => {
                                        current_room_id = Some(room_id);
                                    }
                                    Err(e) => {
                                        send_error(session.clone(), &e).await;
                                    }
                                }
                            } else {
                                send_error(session.clone(), "Not logged in").await;
                            }
                           
                        }

                        Err(e) => {
                            send_error(session.clone(), &format!("Invalid message: {}", e)).await;
                        }
                    }
                }

                Ok(AggregatedMessage::Ping(msg)) => {
                    let mut s = session.write().await;
                    let _ = s.pong(&msg).await;
                }

                _ => {}
            }
        }

        // Connection closed - cleanup
        if let (Some(room_id), Some(uid)) = (current_room_id, user_id) {
            let _ = update_player_connected(rooms, room_id, uid,false).await;
        }
    });

    Ok(res)
}

async fn join_room(
    rooms: Arc<RwLock<std::collections::HashMap<String, Room>>>,
    session: Arc<RwLock<actix_ws::Session>>,
    room_id: String,
    user_id: String,
    username: String,
) -> Result<(), String> {
    println!("🎮 join_room called: room={}, user={}", room_id, user_id);
    
    let symbol;
    let should_start_game;
    let game_state;
    
    // Scope 1: Modify room state (hold write lock briefly)
    {
        let mut rooms_lock = rooms.write().await;
        println!("🔒 Acquired write lock on rooms");
        
        let room = rooms_lock.get_mut(&room_id).ok_or("Room not found")?;

        // Create player
        let player = Player {
            user_id: user_id.clone(),
            username: username.clone(),
            symbol: crate::game::Symbol::X, // Will be set by add_player
            session: session.clone(),
            connected: true,
        };

        symbol = if room.players.is_empty() {
            crate::game::Symbol::X
        } else {
            crate::game::Symbol::O
        };

        // Add player to room
        room.add_player(player)?;
        println!("✅ Player added to room");

        // Check if game should start
        should_start_game = room.players.len() == 2;
        game_state = room.game_state.clone();
        
        // Lock is released here automatically when rooms_lock goes out of scope
    }


    println!("🔓 Released write lock on rooms");

   
    // Scope 2: Send messages (no room lock held)
    {
        let mut rooms_lock = rooms.write().await;
        println!("🔒 Acquired write lock on rooms");
        
        let room = rooms_lock.get_mut(&room_id).ok_or("Room not found")?;

        let players_info = room.players.iter().map(|p| PlayerInfo {
            username: p.username.clone(),
            symbol: p.symbol,
        }).collect::<Vec<PlayerInfo>>();
        // Send confirmation to joining player
        let event = ServerEvent::RoomJoined {
            room_id: room_id.clone(),
            your_symbol: symbol,
            game_state: game_state.clone(),
            players: players_info,
        };
        
        let mut s = session.write().await;
        s.text(event.to_json()).await.map_err(|e| e.to_string())?;
        println!("📤 Sent RoomJoined event to player");
    }
    
    // Scope 3: Broadcast to other players (acquire read lock briefly)
    {
        let rooms_lock = rooms.read().await;
        let room = rooms_lock.get(&room_id).ok_or("Room not found")?;
        
        let broadcast_event = ServerEvent::PlayerJoined {
            username: username.clone(),
            symbol,
        };
        room.broadcast(&broadcast_event.to_json()).await;
        println!("📢 Broadcasted PlayerJoined event");
        
        // If room is now full, start game
        if should_start_game {
            println!("🎮 Starting game!");
            let start_event = ServerEvent::GameStarted {
                game_state,
            };
            room.broadcast(&start_event.to_json()).await;
            println!("📢 Broadcasted GameStarted event");
        }
    }

    println!("✅ join_room completed successfully");
    Ok(())
}


async fn update_player_connected(
    rooms: Arc<RwLock<std::collections::HashMap<String, Room>>>,
    room_id: String,
    user_id: String,
    connected: bool,
) -> Result<(), String> {
    let mut rooms_lock = rooms.write().await;
    let room = rooms_lock.get_mut(&room_id).ok_or("Room not found")?;

    let player = room.get_player_mut(&user_id).ok_or("Not in this room")?;
    player.connected = connected;
    Ok(())
}
   
async fn make_move(
    rooms: Arc<RwLock<std::collections::HashMap<String, Room>>>,
    room_id: String,
    user_id: String,
    row: usize,
    col: usize,
) -> Result<(), String> {
    println!("🎯 make_move: room={}, user={}, pos=({},{})", room_id, user_id, row, col);
    
    let username;
    let symbol;
    let game_ended;
    let game_state;
    let winner_username;
    
    // Scope 1: Make the move (hold write lock briefly)
    {
        let mut rooms_lock = rooms.write().await;
        let room = rooms_lock.get_mut(&room_id).ok_or("Room not found")?;

        let player = room.get_player(&user_id).ok_or("Not in this room")?;
        symbol = player.symbol;
        username = player.username.clone();

        // Make the move
        game_ended = room.game_state.make_move(row, col, symbol,user_id)?;
        game_state = room.game_state.clone();
        
        // Get winner info if game ended
        winner_username = if game_ended {
            match &room.game_state.result {
                Some(crate::game::GameResult::Win(winner_id)) => {
                    room.get_player(winner_id).map(|p| p.username.clone())
                }
                _ => None,
            }
        } else {
            None
        };
        
        // Write lock released here
    }
    
    // Scope 2: Broadcast the move (use read lock)
    {
        let rooms_lock = rooms.read().await;
        let room = rooms_lock.get(&room_id).ok_or("Room not found")?;
        
        let event = ServerEvent::MoveMade {
            username: username.clone(),
            row,
            col,
            symbol,
            game_state: game_state.clone(),
        };
        room.broadcast(&event.to_json()).await;
        println!("📢 Broadcasted MoveMade event");

        // If game ended, broadcast game over
        if game_ended {
            let game_over_event = ServerEvent::GameOver {
                winner: winner_username,
                game_state,
            };
            room.broadcast(&game_over_event.to_json()).await;
            println!("📢 Broadcasted GameOver event");
        }
    }

    Ok(())
}

async fn leave_room(
    rooms: Arc<RwLock<std::collections::HashMap<String, Room>>>,
    room_id: String,
    user_id: String,
) -> Result<(), String> {
    let username;
    {
        let mut rooms_lock = rooms.write().await;
        let room = rooms_lock.get_mut(&room_id).ok_or("Room not found")?;
        
        username = room.get_player(&user_id)
            .map(|p| p.username.clone())
            .ok_or("Player not in room")?;
        
        _ = room.remove_player(user_id.clone());

        if room.players.len() == 0 {
            rooms_lock.remove(&room_id);
            println!("🔓 Removed room {} from rooms", room_id);
        }

        println!("✅ Removed player {} from room", username);
    }
    
    // Scope 2: Broadcast to remaining players
    {
        let rooms_lock = rooms.read().await;
        if let Some(room) = rooms_lock.get(&room_id) {
            let broadcast_event = ServerEvent::PlayerLeft { 
                username: username.clone()  // Use username, not user_id!
            };
            room.broadcast(&broadcast_event.to_json()).await;
            println!("📢 Broadcasted PlayerLeft event");
        }
    }
    Ok(())
}

async fn send_error(session: Arc<RwLock<actix_ws::Session>>, message: &str) {
    let event = ServerEvent::Error {
        message: message.to_string(),
    };
    let mut s = session.write().await;
    let _ = s.text(event.to_json()).await;
}