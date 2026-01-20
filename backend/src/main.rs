use anyhow::Result;
use actix_web::{web, HttpServer};
use actix_cors::Cors;

pub mod routes;
pub mod middleware;
pub mod game;
pub mod events;
pub mod ws_handler;

use actix_web::{http};

use db::{Store};
use routes::user::{create_user,sign_in,get_user};

use crate::game::AppState;
use crate::routes::room::{create_room, get_rooms};

async fn health_check() -> web::Json<String> {
    web::Json("OK".to_string())
}

#[actix_web::main]
async fn main() -> Result<()> {
    let app_state = web::Data::new(AppState::new());
    dotenvy::dotenv().ok();
    let store = Store::new().await?;
    HttpServer::new(move || {

        let cors = Cors::default()
            .allowed_origin("http://localhost:3000")
            .allowed_methods(vec!["GET", "POST"])
            .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT])
            .allowed_header(http::header::CONTENT_TYPE)
            .max_age(3600);

        actix_web::App::new()
        .wrap(cors)
        .service(web::scope("/api/v1")
            .service(web::resource("/health").route(web::get().to(health_check)))
            .service(web::resource("/signup").route(web::post().to(create_user)))
            .service(web::resource("/signin").route(web::post().to(sign_in)))
            .service(web::resource("/me").route(web::get().to(get_user)))
            .service(web::resource("/create_room").route(web::post().to(create_room)))
            .service(web::resource("/ws").route(web::get().to(ws_handler::ws_handler)))
            .service(web::resource("/rooms").route(web::get().to(get_rooms)))
            .app_data(web::Data::new(store.clone())))
            .app_data(app_state.clone())

    })
    .bind("0.0.0.0:8080")?
    .run()
    .await?;

    Ok(())
}
