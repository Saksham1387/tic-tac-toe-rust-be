use::serde::{Serialize,Deserialize};
use sqlx::{Type, types::time::OffsetDateTime};
use anyhow::{Ok, Result};
use crate::Store;
use rand::{thread_rng, Rng};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "room_status", rename_all = "snake_case")]
pub enum RoomStatus {
    Waiting,
    InProgress,
    Completed
}

#[derive(Serialize,Deserialize,Debug)]
pub struct Room {
    pub id: Uuid,
    pub room_name:String,
    pub max_spectators:Option<i32>,
    pub max_players: i32,
    pub is_private:Option<bool>,
    pub room_code:String,
    pub status:Option<RoomStatus>,
    pub created_by: Option<Uuid>,
    pub created_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "room_role", rename_all = "snake_case")]
pub enum RoomRole {
    Player,
    Spectator
}

#[derive(Serialize,Deserialize)]
pub struct RoomParticipant {
    id:Uuid,
    room_id:Uuid,
    user_id:Uuid,
    role:RoomRole,
    position:Option<i32>,
    joined_at:Option<OffsetDateTime>
}

#[derive(Serialize,Deserialize)]
pub struct CreateRoomRequest {
    pub room_name:String,
    pub is_private:bool,
    pub max_spectators:i32
}

#[derive(Serialize,Deserialize)]
pub struct CreateRoomResponse {
    pub room_id:Uuid,
    pub room_code:String
}

#[derive(Serialize,Deserialize)]
pub struct GetRoomsRequest {
    pub status:Option<RoomStatus>
}

#[derive(Serialize,Deserialize)]
pub struct GetRoomsRespose {
    pub rooms:Vec<Room>
}

#[derive(Serialize,Deserialize)]
pub struct JoinRoomRequest {
    pub room_id:Uuid,
    pub role:RoomRole,
    pub user_id:Uuid,
    pub position:Option<i32>
}

pub struct JoinRoomResponse {
    pub room_id:Uuid,
    pub role:RoomRole,
    pub position:Option<i32>
} 

fn generate_code() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    const CODE_LEN: usize = 5;

    let mut rng = thread_rng();

    (0..CODE_LEN)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

impl Store {
    pub async fn create_room(&self,request:CreateRoomRequest) -> Result<CreateRoomResponse> {
        let code = generate_code();
        let room = sqlx::query_as!(Room,r#"
            INSERT INTO rooms (
                room_name,
                is_private,
                max_spectators,
                room_code
            )
            VALUES ($1, $2, $3, $4)
            RETURNING
                id,
                room_name,
                max_spectators,
                max_players,
                is_private,
                room_code,
                status as "status: RoomStatus",
                created_by,
                created_at
            "#,request.room_name,request.is_private,request.max_spectators,code)
            .fetch_one(&self.pool)
            .await?;

        print!("{:?}",room);
        Ok(CreateRoomResponse {
            room_id:room.id,
            room_code:room.room_code
        })
    }

    pub async fn get_rooms(&self,request:GetRoomsRequest) -> Result<GetRoomsRespose> {
 
        match request.status {
            Some(status) => {

                let rooms:Vec<Room> = sqlx::query_as!(Room,r#"
                    SELECT  

                        id,
                        room_name,
                        max_spectators,
                        max_players,
                        is_private,
                        room_code,
                        status as "status: RoomStatus",
                        created_by,
                        created_at
                    
                    FROM rooms 

                    WHERE status = $1
                "#, status as RoomStatus).fetch_all(&self.pool).await?;

                return Ok(GetRoomsRespose {
                    rooms:rooms
                })

            }

            None => {
                let rooms:Vec<Room> = sqlx::query_as!(Room,r#"
                    SELECT  

                    id,
                    room_name,
                    max_spectators,
                    max_players,
                    is_private,
                    room_code,
                    status as "status: RoomStatus",
                    created_by,
                    created_at
                    
                    FROM rooms 
                "#).fetch_all(&self.pool).await?;

                return Ok(GetRoomsRespose {
                    rooms:rooms
                })

            }
        }
    }

    pub async fn join_room(&self,request:JoinRoomRequest) -> Result<JoinRoomResponse> {
        let joined = sqlx::query_as!(RoomParticipant,r#"
        INSERT INTO room_participants (room_id, user_id, role, position)
        VALUES ($1, $2, $3, $4)
        RETURNING
            id,
            room_id,
            user_id,
            position,
            role as "role: RoomRole",
            joined_at
        "#,request.room_id,request.user_id,request.role as RoomRole,request.position)
            .fetch_one(&self.pool)
            .await?;


        Ok(JoinRoomResponse{
            room_id:joined.room_id,
            role:joined.role,
            position:joined.position
        })
    }

}



