use actix_web::{App, HttpServer};
use actix_cors::Cors;
use sqlx::PgPool;
use actix_web::web::Data;
mod routes;
use routes::user::{get_all_users, create_user, update_user, delete_user};
use routes::auth::{check_user, login};
use routes::me::me;

// Models and user routes moved to routes/user.rs
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL harus di-set");
    let db_pool = PgPool::connect(&db_url).await.expect("Gagal connect ke database");

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(db_pool.clone()))
            .wrap(
                Cors::default()
                    .allowed_origin("http://localhost:5173")
                    .allow_any_method()
                    .allow_any_header()
                    .supports_credentials()
            )
            .service(get_all_users)
            .service(create_user)
            .service(update_user)
            .service(delete_user)
            .service(check_user)
            .service(login)
            .service(me)
    })
    .bind(("localhost", 8080))?
    .run()
    .await
}
