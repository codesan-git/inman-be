use actix_web::{dev::Payload, Error, FromRequest, HttpRequest};
use futures::future::{ready, Ready};
use crate::middleware::jwt_middleware::Claims;
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};

impl FromRequest for Claims {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        // Ambil token dari header Authorization atau cookie 'token'
        let token_opt = req
            .headers()
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|auth| {
                if auth.starts_with("Bearer ") {
                    Some(auth[7..].to_string())
                } else {
                    None
                }
            })
            .or_else(|| {
                req.cookie("token").map(|c| c.value().to_string())
            });

        if let Some(token) = token_opt {
            let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET harus di-set");
            let decoding_key = DecodingKey::from_secret(secret.as_bytes());
            let validation = Validation::new(Algorithm::HS256);
            match decode::<Claims>(&token, &decoding_key, &validation) {
                Ok(data) => ready(Ok(data.claims)),
                Err(_) => ready(Err(actix_web::error::ErrorUnauthorized(serde_json::json!({"message": "Invalid JWT"})))),

            }
        } else {
            ready(Err(actix_web::error::ErrorUnauthorized(serde_json::json!({"message": "No JWT token"}))))
        }
    }
}
