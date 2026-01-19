use actix_web::{web, Error, HttpRequest, HttpResponse};
use actix_ws::AggregatedMessage;
use futures_util::StreamExt;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};

use crate::game::{AppState, Player, Symbol, User};
use crate::events::{ClientEvent, PlayerInfo, ServerEvent};
use crate::game::RoomManager;
use crate::middleware::JwtClaims;

pub async fn ws_handler(
    req: HttpRequest,
    stream: web::Payload,
    app_state: web::Data<AppState>,
    claims: JwtClaims
) -> Result<HttpResponse, Error> {
    println!("🔌 New WebSocket connection");
    
    let (res, session, stream) = actix_ws::handle(&req, stream)?;
    

    println!("✅ WebSocket handshake successful!");

    // Create channel for this user
    let (tx, mut rx) = mpsc::channel::<String>(32);
    
    let mut stream = stream
        .aggregate_continuations()
        .max_continuation_size(2_usize.pow(20));

    let room_manager = app_state.room_manager.clone();
    
    // Clone session for sender task
    let mut session_clone = session.clone();
    let mut session_sender = session.clone();     

    let user_id = claims.0.sub;
      // Replace with actual auth
    room_manager.write().await.clients.insert(
        user_id.clone(),
        User { tx: tx.clone() }
    );
    // TASK 1: Receiver - Read from WebSocket, process messages
    actix_web::rt::spawn(async move {
        let mut current_room_id: Option<String> = None;
        let mut user_id: Option<String> = None;

        println!("🎯 Receiver task started");

        while let Some(msg) = stream.next().await {
            match msg {
                Ok(AggregatedMessage::Text(text)) => {
                    println!("📨 Received text: {}", text);
                    
                    let event: Result<ClientEvent, _> = serde_json::from_str(&text);

                    match event {
                        Ok(ClientEvent::JoinRoom { room_id, user_id: uid, username }) => {
                            println!("🎮 Join room: {}, user: {}", room_id, uid);
                            
                            user_id = Some(uid.clone());
                            
                            let result = join_room(
                                room_manager.clone(),
                                room_id.clone(),
                                uid.clone(),
                                username,
                            ).await;

                            match result {
                                Ok(response_msg) => {
                                    println!("✅ Joined room");
                                    current_room_id = Some(room_id);
                                    
                                    // Send response through channel
                                    if let Some(user) = room_manager.read().await.clients.get(&uid) {
                                        let _ = user.tx.send(response_msg).await;
                                    }
                                }
                                Err(e) => {
                                    println!("❌ Join failed: {}", e);
                                    send_error_channel(&room_manager, &uid, &e).await;
                                }
                            }
                        }

                        Ok(ClientEvent::MakeMove { room_id, row, col }) => {
                            if let Some(uid) = &user_id {
                                let _ = make_move(
                                    room_manager.clone(),
                                    room_id,
                                    uid.clone(),
                                    row,
                                    col,
                                ).await;
                            }
                        }

                        Ok(ClientEvent::LeaveRoom { room_id }) => {
                            if let Some(uid) = &user_id {
                                let _ = leave_room(
                                    room_manager.clone(),
                                    room_id.clone(),
                                    uid.clone(),
                                ).await;
                                current_room_id = None;
                            }
                        }

                        Err(e) => {
                            println!("❌ Parse error: {}", e);
                            if let Some(uid) = &user_id {
                                send_error_channel(&room_manager, uid, &format!("Invalid message: {}", e)).await;
                            }
                        }
                    }
                }

                Ok(AggregatedMessage::Ping(msg)) => {
                    println!("🏓 PING");
                    let _ = session_clone.pong(&msg).await;
                }

                Ok(AggregatedMessage::Close(reason)) => {
                    println!("🔌 Client closed: {:?}", reason);
                    break;
                }

                Err(e) => {
                    println!("❌ Stream error: {}", e);
                    break;
                }

                _ => {}
            }
        }

        println!("🔌 Receiver task ending");
        
        // Cleanup
        if let (Some(room_id), Some(uid)) = (current_room_id, user_id.clone()) {
            let _ = update_player_connected(room_manager.clone(), room_id, uid.clone()).await;
            
            // Remove from clients
            room_manager.write().await.clients.remove(&uid);
        }
    });

    // TASK 2: Sender - Read from channel, write to WebSocket
    actix_web::rt::spawn(async move {
        println!("📤 Sender task started");
        
        while let Some(message) = rx.recv().await {
            println!("📤 Sending: {}", message);
            if let Err(e) = session_sender.text(message).await {
                println!("❌ Send failed: {}", e);
                break;
            }
        }
        
        println!("📤 Sender task ending");
    });

    Ok(res)
}

async fn join_room(
    room_manager: Arc<RwLock<RoomManager>>,
    room_id: String,
    user_id: String,
    username: String,
) -> Result<String, String> {
    println!("🎮 join_room called: room={}, user={}", room_id, user_id);
    
    let symbol;
    let should_start_game;
    let game_state; 
    let players_info;
    
    {
        let mut rm = room_manager.write().await;

        rm.subscriptions.entry(room_id.clone()).or_insert_with(HashSet::new).insert(user_id.clone());

        let room = rm.rooms.get_mut(&room_id).ok_or("Room not found")?;

        let player = Player {
            user_id: user_id.clone(),
            username: username.clone(),
            symbol: Symbol::X,  // Will be set by add_player
            connected: true,
        };

        symbol = if room.players.is_empty() {
            Symbol::X
        } else {
            Symbol::O
        };

        room.add_player(player)?;

        should_start_game = room.players.len() == 2;
        game_state = room.game_state.clone();

        players_info = room.players.iter().map(|p| PlayerInfo {
            username: p.username.clone(),
            symbol: p.symbol,
        }).collect();
    }

    
    let event = ServerEvent::RoomJoined {
        room_id: room_id.clone(),
        your_symbol: symbol,
        game_state: game_state.clone(),
        players:players_info
    };

    {
        let rm = room_manager.read().await;
        let broadcast_event = ServerEvent::PlayerJoined {
            username: username.clone(),
            symbol,
        };
        rm.broadcast(&room_id, broadcast_event.to_json()).await;
        
        if should_start_game {
            let start_event = ServerEvent::GameStarted { game_state };
            rm.broadcast(&room_id, start_event.to_json()).await;
        }
    }

    Ok(event.to_json())
}


async fn update_player_connected(
    room_manager: Arc<RwLock<RoomManager>>,
    room_id: String,
    user_id: String,
) -> Result<(), String> {
    let mut rm = room_manager.write().await;
    rm.subscriptions.entry(room_id.clone()).and_modify(|set| { set.remove(&user_id); });

    
    Ok(())
}
   
async fn make_move(
    room_manager: Arc<RwLock<RoomManager>>,
    room_id: String,
    user_id: String,
    row: usize,
    col: usize,
) -> Result<String, String> {
    println!("🎯 make_move: room={}, user={}, pos=({},{})", room_id, user_id, row, col);
    
    let username;
    let symbol;
    let game_ended;
    let game_state;
    let winner_username;
    
    {
        let mut rm = room_manager.write().await;
        let room = rm.rooms.get_mut(&room_id).ok_or("Room not found")?;

        let player = room.get_player(&user_id).ok_or("Not in this room")?;
        symbol = player.symbol;
        username = player.username.clone();

        game_ended = room.game_state.make_move(row, col, symbol,user_id)?;
        game_state = room.game_state.clone();
        
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
    }
    let event  = ServerEvent::MoveMade { username:username.clone(), row, col, symbol, game_state:game_state.clone() };

 
    {
        let rm = room_manager.read().await;

        rm.broadcast(&room_id,event.to_json()).await;
        println!("📢 Broadcasted MoveMade event");

        if game_ended {
            let game_over_event = ServerEvent::GameOver {
                winner: winner_username,
                game_state,
            };
            rm.broadcast(&room_id,game_over_event.to_json()).await;
            println!("📢 Broadcasted GameOver event");
        }
    }

    Ok(event.to_json())
}

async fn leave_room(
    room_manager: Arc<RwLock<RoomManager>>,
    room_id: String,
    user_id: String,
) -> Result<(), String> {
    let username;
    {
        let mut rm: tokio::sync::RwLockWriteGuard<'_, RoomManager> = room_manager.write().await;
        
        let room = rm.rooms.get_mut(&room_id).ok_or("Room not found")?;
        
        username = room.get_player(&user_id)
            .map(|p| p.username.clone())
            .ok_or("Player not in room")?;
        
        _ = room.remove_player(user_id.clone());

        if room.players.len() == 0 {
            rm.rooms.remove(&room_id);
            println!("🔓 Removed room {} from rooms", room_id);
        }

        println!("✅ Removed player {} from room", username);
    }
    
    // Scope 2: Broadcast to remaining players
    {
        let rm = room_manager.read().await;
        if let Some(_) = rm.rooms.get(&room_id) {
            let broadcast_event = ServerEvent::PlayerLeft { 
                username: username.clone()  // Use username, not user_id!
            };
            rm.broadcast(&room_id,broadcast_event.to_json()).await;
            println!("📢 Broadcasted PlayerLeft event");
        }
    }
    Ok(())
}

async fn send_error_channel(
    room_manager: &Arc<RwLock<RoomManager>>,
    user_id: &str,
    message: &str,
) {
    let event = ServerEvent::Error {
        message: message.to_string(),
    };
    
    let rm = room_manager.read().await;
    if let Some(user) = rm.clients.get(user_id) {
        let _ = user.tx.send(event.to_json()).await;
    }
}
