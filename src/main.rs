use actix_web::{App, HttpServer, middleware::Logger};

use actix_cors::Cors;
use sqlx::PgPool;
use actix_web::web::Data;
mod routes;
mod middleware;
mod services;
use routes::user::user_config;
use routes::auth::{check_user, login, logout};
use routes::me::me;
use routes::items::items_config;
use routes::upload::upload_config;
use routes::permissions::permissions_config;
use routes::borrowings::borrowings_config;
use services::drive_storage::{DriveConfig, DriveClient, GoogleCredentials, create_drive_client, ensure_folder_exists};
use std::sync::Arc;
use std::path::Path;
use tokio::sync::Mutex;

// Models and user routes moved to routes/user.rs
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();
    let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL harus di-set");
    let db_pool = PgPool::connect(&db_url).await.expect("Gagal connect ke database");
    
    let port = std::env::var("PORT")
    .ok()
    .and_then(|p| p.parse().ok())
    .unwrap_or(8080);

    let frontend_urls = Arc::new(vec![
        std::env::var("FRONTEND_URL").expect("FRONTEND_URL harus di-set"),
        "http://localhost:5173".to_string(),
    ]);
    
    // Konfigurasi upload file
    let upload_dir = Path::new(&std::env::var("UPLOAD_DIR").unwrap_or_else(|_| "uploads".to_string())).to_path_buf();
    
    // Pastikan direktori upload ada (untuk file statis)
    if !upload_dir.exists() {
        std::fs::create_dir_all(&upload_dir).expect("Gagal membuat direktori upload");
    }
    
    // Konfigurasi BASE_URL untuk akses API
    let base_url = std::env::var("BASE_URL").unwrap_or_else(|_| {
        format!("http://localhost:{}", port)
    });
    println!("[INFO] Menggunakan BASE_URL: {}", base_url);
    
    // Konfigurasi Google Drive
    let mut drive_config = DriveConfig::default();
    drive_config.base_url = base_url.clone();
    
    // Inisialisasi Drive client
    let drive_client = match create_drive_client(&drive_config).await {
        Ok(mut client) => {
            // Pastikan folder untuk upload sudah ada
            let folder_id = match ensure_folder_exists(&mut client, &drive_config.folder_id).await {
                Ok(id) => {
                    println!("Berhasil terhubung ke Google Drive. Folder ID: {}", id);
                    // Update folder_id jika berbeda
                    if drive_config.folder_id != id {
                        println!("Folder ID diperbarui: {}", id);
                    }
                    id
                },
                Err(e) => {
                    eprintln!("Warning: Gagal memastikan folder ada: {}", e);
                    eprintln!("Pastikan kredensial Google Drive sudah benar.");
                    eprintln!("Upload akan fallback ke penyimpanan lokal jika Google Drive tidak tersedia.");
                    drive_config.folder_id.clone()
                }
            };
            
            // Buat DriveConfig baru dengan folder_id yang diperbarui
            let mut updated_config = drive_config.clone();
            updated_config.folder_id = folder_id;
            
            client
        },
        Err(e) => {
            eprintln!("Warning: Gagal membuat Google Drive client: {}", e);
            eprintln!("Pastikan kredensial Google Drive sudah benar.");
            eprintln!("Upload akan fallback ke penyimpanan lokal jika Google Drive tidak tersedia.");
            
            // Buat dummy client kosong untuk fallback
            match create_drive_client(&DriveConfig {
                credentials_json: "{}".to_string(),
                ..DriveConfig::default()
            }).await {
                Ok(client) => client,
                Err(e) => {
                    eprintln!("Warning: Gagal membuat dummy client untuk Google Drive: {}", e);
                    // Buat client dengan credentials kosong tanpa parsing JSON
                    DriveClient {
                        client: reqwest::Client::new(),
                        credentials: GoogleCredentials {
                            r#type: "".to_string(),
                            project_id: "".to_string(),
                            private_key_id: "".to_string(),
                            private_key: "".to_string(),
                            client_email: "".to_string(),
                            client_id: "".to_string(),
                            auth_uri: "".to_string(),
                            token_uri: "".to_string(),
                            auth_provider_x509_cert_url: "".to_string(),
                            client_x509_cert_url: "".to_string(),
                        },
                        access_token: None,
                        token_expiry: None,
                    }
                }
            }
        }
    };
    
    let drive_client = Arc::new(Mutex::new(drive_client));
    
    let cors_urls = frontend_urls.clone();
    let upload_dir_for_server = upload_dir.clone();

    HttpServer::new(move || {
        let cors_urls = cors_urls.clone();
        let upload_dir = upload_dir_for_server.clone();

        App::new()
            .app_data(Data::new(db_pool.clone()))
            .app_data(Data::new(drive_config.clone()))
            .app_data(Data::new(drive_client.clone()))
            .wrap(Logger::default())
            // Konfigurasi CORS
            .wrap(
                Cors::default()
                .allowed_origin_fn(move |origin, _req_head| {
                    cors_urls.iter().any(|url| origin.as_bytes() == url.as_bytes())
                })
                    .allow_any_method()
                    .allow_any_header()
                    .supports_credentials()
            )
            // Konfigurasi untuk menyajikan file statis dari direktori uploads
            // Dengan header CORS yang benar
            .service(
                actix_files::Files::new("/uploads", &upload_dir)
                    .use_last_modified(true)
                    .use_etag(true)
                    .prefer_utf8(true)
                    .disable_content_disposition()
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
                actix_web::web::scope("/api/lookup")
                    .configure(routes::lookup::lookup_config)
            )
            .service(
                actix_web::web::scope("/api/me")
                                        .service(me)
            )
            .service(
                actix_web::web::scope("/api/upload")
                    .configure(upload_config)
            )
            .service(
                actix_web::web::scope("/api/permissions")
                    .configure(permissions_config)
            )
            .service(
                actix_web::web::scope("/api/borrowings")
                    .configure(borrowings_config)
            )
    })
    .bind(("0.0.0.0", port))?    
    .run()
    .await
}
