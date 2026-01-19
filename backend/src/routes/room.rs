use actix_web::{web, Result};
use db::Store;
use db::models::room::{CreateRoomRequest, CreateRoomResponse, GetRoomsRequest, GetRoomsRespose};

use crate::game::AppState;
use crate::middleware::JwtClaims;


pub async fn create_room(data: web::Data<Store>,data1:web::Data<AppState>,claims: JwtClaims,request: web::Json<CreateRoomRequest>) -> Result<web::Json<CreateRoomResponse>> {
    let store = data.into_inner();
    let app_store1 = data1.into_inner();
    let mut rm: tokio::sync::RwLockWriteGuard<'_, crate::game::RoomManager> = app_store1.room_manager.write().await;

    let room_name = request.room_name.clone();
    let room = store.create_room(request.into_inner()).await.map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;
    
    rm.rooms.insert(
        room.room_id.to_string(),
        crate::game::Room::new(room.room_id.to_string(), room_name)
    );
    Ok(web::Json(room))
}


pub async fn get_rooms(data: web::Data<Store>,claims: JwtClaims,request: web::Json<GetRoomsRequest> ) ->  Result<web::Json<GetRoomsRespose>> {
    let store = data.into_inner();
    let rooms = store.get_rooms(request.into_inner()).await.map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;

    Ok(web::Json(rooms))
}

