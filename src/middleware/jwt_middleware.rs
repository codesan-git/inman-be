use actix_web::{dev::ServiceRequest, Error};
use actix_web::HttpMessage;
use actix_web::error::ErrorUnauthorized;
use actix_web::dev::{Service, ServiceResponse, Transform};
use futures::future::{ok, Ready, LocalBoxFuture};
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use serde::Deserialize;
use std::rc::Rc;
use std::task::{Context, Poll};

#[derive(Debug, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub role: String,
}

pub struct JwtMiddleware {
    pub secret: Rc<String>,
}

impl JwtMiddleware {
    pub fn new(secret: String) -> Self {
        JwtMiddleware { secret: Rc::new(secret) }
    }
}

impl<S, B> Transform<S, ServiceRequest> for JwtMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + Clone + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = JwtMiddlewareMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(JwtMiddlewareMiddleware {
            service,
            secret: Rc::clone(&self.secret),
        })
    }
}

#[derive(Clone)]
pub struct JwtMiddlewareMiddleware<S> {
    service: S,
    secret: Rc<String>,
}

impl<S, B> Service<ServiceRequest> for JwtMiddlewareMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + Clone + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
    let secret = Rc::clone(&self.secret);
    let service = self.service.clone();
    Box::pin(async move {
        let (req, payload) = req.into_parts();
        let headers = req.headers().clone();
        let mut token_opt: Option<String> = None;
        // Cek Authorization header
        let auth_header = headers.get("Authorization").and_then(|h| h.to_str().ok());
        if let Some(auth) = auth_header {
            if auth.starts_with("Bearer ") {
                token_opt = Some(auth[7..].to_string());
            }
        }
        // Jika tidak ada di header, cek cookie 'token'
        if token_opt.is_none() {
            if let Some(cookie_header) = headers.get("cookie").and_then(|h| h.to_str().ok()) {
                for cookie in cookie_header.split(';') {
                    let cookie = cookie.trim();
                    if cookie.starts_with("token=") {
                        token_opt = Some(cookie[6..].to_string());
                        break;
                    }
                }
            }
        }
        if let Some(token) = token_opt {
            println!("[JwtMiddleware] Token found: {}", token);
            let validation = Validation::new(Algorithm::HS256);
            let decoding_key = DecodingKey::from_secret(secret.as_bytes());
            let token_data = decode::<Claims>(&token, &decoding_key, &validation);
            if let Err(e) = &token_data {
                println!("[JwtMiddleware] JWT decode error: {}", e);
            }
            if let Ok(data) = token_data {
                println!("[JwtMiddleware] JWT valid, forwarding request...");
                req.extensions_mut().insert(data.claims);
                let req = ServiceRequest::from_parts(req, payload);
                let resp = service.call(req).await;
                println!("[JwtMiddleware] Request forwarded, response sent.");
                return resp;
            }
        } else {
            println!("[JwtMiddleware] No token found in header or cookie");
        }
        Err(ErrorUnauthorized("Unauthorized: Invalid or missing JWT"))
    })
}
}
