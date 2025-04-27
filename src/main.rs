use actix_web::{App, HttpServer, middleware::Logger};

use actix_cors::Cors;
use sqlx::PgPool;
use actix_web::web::Data;
mod routes;
mod middleware;
use routes::user::user_config;
use routes::auth::{check_user, login, logout};
use routes::me::me;
use routes::items::items_config;

// Models and user routes moved to routes/user.rs
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL harus di-set");
    let db_pool = PgPool::connect(&db_url).await.expect("Gagal connect ke database");

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(db_pool.clone()))
            .wrap(Logger::default())
            .wrap(
                Cors::default()
                    .allowed_origin("http://localhost:5173")
                    .allow_any_method()
                    .allow_any_header()
                    .supports_credentials()
            )
            
            .service(check_user)
            .service(login)
            .service(logout)
            .service(me)
            .service(
                actix_web::web::scope("/api/users")
                                        .configure(user_config)
            )
            .service(
                actix_web::web::scope("/api/items")
                    .configure(items_config)
            )
            .service(
                actix_web::web::scope("/api/me")
                                        .service(me)
            )
    })
    .bind(("localhost", 8080))?
    .run()
    .await
}
