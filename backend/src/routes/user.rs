use std::env;

use actix_web::{web, Result};
use db::Store;
use db::models::user::{CreateUserRequest, CreateUserResponse, GetUserRequest, UserSigninRequest};
use serde::{Deserialize, Serialize};
use jsonwebtoken::{EncodingKey, Header,encode};

use crate::middleware::JwtClaims;


#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize
}

impl Claims {
    pub fn new(sub:String) -> Self {
        Self { sub , exp: 1000000000000000000}
    }
}

#[derive(Serialize, Deserialize)]
pub struct SignInResponse {
    pub token: String,
}

#[derive(Serialize, Deserialize)]
pub struct GetUserResponse {
    pub username: String,
    pub email:String
}

pub async fn get_user(data: web::Data<Store>, claims: JwtClaims) -> Result<web::Json<GetUserResponse>> {
    let store = data.into_inner();
    let user = store.get_user_by_id(claims.0.sub).await.map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;
    Ok(web::Json(GetUserResponse { username: user.user.username,email:user.user.email }))
}

pub async fn create_user(data: web::Data<Store>,request:web::Json<CreateUserRequest>) -> Result<web::Json<CreateUserResponse>> {
    let store = data.into_inner();
    let user = store.create_user(request.into_inner()).await.map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;
    Ok(web::Json(user))
}

pub async fn sign_in(data: web::Data<Store>, request: web::Json<UserSigninRequest>) -> Result<web::Json<SignInResponse>> {
    let store = data.into_inner();
    let user = store.get_user(GetUserRequest { email: request.into_inner().email }).await.map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;
    let claim = Claims::new(user.user.id);
    let token = encode(&Header::default(), &claim, &EncodingKey::from_secret(env::var("SECRET_KEY").unwrap().as_bytes())).map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?;
    Ok(web::Json(SignInResponse { token: token }))
}

