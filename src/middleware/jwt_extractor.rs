use actix_web::{dev::Payload, Error, FromRequest, HttpRequest};
use futures::future::{ready, Ready};
#[derive(Debug, serde::Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    #[allow(dead_code)]
    pub exp: usize,
    pub role: String,
}

use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};

impl FromRequest for Claims {
    // DEBUG: log every extractor call
    // (inserted in from_request below)

    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
    println!("[JWT Extractor] called for path: {}", req.path());
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
        println!("[JWT Extractor] Token found: {}", &token);
            let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET harus di-set");
            let decoding_key = DecodingKey::from_secret(secret.as_bytes());
            let validation = Validation::new(Algorithm::HS256);
            match decode::<Claims>(&token, &decoding_key, &validation) {
                Ok(data) => {
                    println!("[JWT Extractor] JWT decode success: sub={}, role={}", data.claims.sub, data.claims.role);
                    ready(Ok(data.claims))
                },
                Err(e) => {
                    println!("[JWT Extractor] JWT decode error: {:?}", e);
                    ready(Err(actix_web::error::ErrorUnauthorized(serde_json::json!({"message": "Invalid JWT"}))))
                },

            }
        } else {
        println!("[JWT Extractor] No JWT token found in header or cookie");
        ready(Err(actix_web::error::ErrorUnauthorized(serde_json::json!({"message": "No JWT token"}))))
    }
    }
}
