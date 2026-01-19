use actix_web::{
    dev::Payload, error::ErrorUnauthorized, FromRequest, HttpRequest,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use std::{env, future::Ready, future::ready};
use crate::routes::user::Claims;

pub struct JwtClaims(pub Claims);

impl FromRequest for JwtClaims {
    type Error = actix_web::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
                // 1️⃣ Authorization: Bearer <token>
            let bearer_token: Option<&str> = req
            .headers()
            .get("Authorization")
            .and_then(|h| h.to_str().ok());
        // 2️⃣ ?token=<token>
        let query_token: Option<&str> = req
            .query_string()
            .split('&')
            .find_map(|pair| {
                let mut iter = pair.splitn(2, '=');
                match (iter.next(), iter.next()) {
                    (Some("token"), Some(value)) => Some(value),
                    _ => None,
                }
            });

        // 3️⃣ Pick whichever exists
        let token: &str = bearer_token
            .or(query_token)
            .ok_or_else(|| ErrorUnauthorized(
                "JWT token missing in Authorization header or query",
            )).unwrap();

        // 4️⃣ Decode JWT
        let secret = env::var("SECRET_KEY").expect("SECRET_KEY must be set");
        let decoding_key = DecodingKey::from_secret(secret.as_bytes());
        let validation = Validation::default();

        match decode::<Claims>(token, &decoding_key, &validation) {
            Ok(token_data) => ready(Ok(JwtClaims(token_data.claims))),
            Err(e) => {
                eprintln!("JWT decoding error: {:?}", e);
                ready(Err(ErrorUnauthorized("Invalid JWT token")))
            }
        }
    }
}