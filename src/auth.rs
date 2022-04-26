use std::env;

use actix_web::HttpRequest;
use anyhow::{Error, Result};
use chrono::{Duration, Local};
use jsonwebtoken::{decode, DecodingKey, TokenData, Validation};
use jsonwebtoken::{encode, EncodingKey, Header};
use lazy_static::lazy_static;
use rand::{RngCore, SeedableRng};
use serde::{Deserialize, Serialize};

const ACCESS_TOKEN_DURATION_IN_MINUTES: i64 = 20;

lazy_static! {
    static ref JWT_SECRET_KEY: String =
        env::var("JWT_SECRET_KEY").expect("Can't read JWT_SECRET_KEY");
}

#[derive(Deserialize, Serialize)]
pub(crate) struct Claims {
    pub user_id: String,
    pub role_id: String,
    pub exp: i64,
}

pub(crate) fn create_token(user_id: String, role_id: String) -> Result<String> {
    let exp_time = Local::now() + Duration::minutes(ACCESS_TOKEN_DURATION_IN_MINUTES);

    let claims = Claims {
        user_id,
        role_id,
        exp: exp_time.timestamp(),
    };

    Ok(encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET_KEY.as_ref()),
    )?)
}

pub(crate) fn create_refresh_token() -> String {
    let mut b: [u8; 46] = [0; 46];
    let mut rng = rand::rngs::StdRng::from_entropy();

    rng.fill_bytes(&mut b);

    hex::encode(b)
}

pub(crate) fn get_claims(http_request: HttpRequest) -> Result<Option<Claims>> {
    let header = http_request.headers().get("Authorization");

    if header.is_none() {
        return Ok(None);
    }

    let s = header.unwrap().to_str()?;
    decode_claims(s)
}

pub(crate) fn is_expired_error(err: &Error) -> bool {
    if let Some(jwt_error) = err.downcast_ref::<jsonwebtoken::errors::Error>() {
        return match jwt_error.kind() {
            &jsonwebtoken::errors::ErrorKind::ExpiredSignature => true,
            _ => false,
        };
    }

    false
}

fn decode_token(token: &str) -> Result<TokenData<Claims>> {
    Ok(decode::<Claims>(
        &token,
        &DecodingKey::from_secret(JWT_SECRET_KEY.as_ref()),
        &Validation::default(),
    )?)
}

pub(crate) fn decode_claims(token: &str) -> Result<Option<Claims>> {
    let jwt_start_index = "Bearer ".len();
    let jwt = token[jwt_start_index..token.len()].to_string();
    let token_data = decode_token(&jwt)?;
    Ok(Some(token_data.claims))
}
