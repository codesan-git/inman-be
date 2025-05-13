use actix_web::{post, get, web, Responder, HttpResponse, Error, HttpRequest};
use crate::middleware::jwt_extractor::Claims;
use crate::services::drive_storage::{upload_file_handler, DriveConfig, DriveClient};
use actix_multipart::Multipart;
use std::sync::Arc;
use tokio::sync::Mutex;
use reqwest::Client;

#[post("")]
pub async fn upload_image(
    req: HttpRequest,
    _claims: Claims, // Menggunakan underscore untuk menandakan variabel yang sengaja tidak digunakan
    payload: Multipart,
    config: web::Data<DriveConfig>,
    client: web::Data<Arc<Mutex<DriveClient>>>,
) -> impl Responder {
    println!("[INFO] Menerima permintaan upload gambar");
    println!("[DEBUG] Request dari: {:?}", req.peer_addr());
    println!("[DEBUG] Headers: {:?}", req.headers());
    println!("[DEBUG] Config: folder_id={}, public_base_url={}", config.folder_id, config.public_base_url);
    
    // Cek apakah folder_id kosong
    if config.folder_id.is_empty() {
        println!("[ERROR] Google Drive folder ID tidak dikonfigurasi");
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Google Drive folder ID tidak dikonfigurasi"
        }));
    }
    
    // Cek apakah credentials_json valid
    if config.credentials_json == "{}" || config.credentials_json.is_empty() {
        println!("[ERROR] Google Drive credentials tidak dikonfigurasi");
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Google Drive credentials tidak dikonfigurasi"
        }));
    }
    
    println!("[DEBUG] Memulai proses upload file...");
    
    match upload_file_handler(payload, config, client).await {
        Ok(json) => {
            println!("[INFO] Upload berhasil: {:?}", json);
            HttpResponse::Ok()
                .append_header(("Access-Control-Allow-Origin", "*"))
                .append_header(("Access-Control-Allow-Methods", "POST, OPTIONS"))
                .append_header(("Access-Control-Allow-Headers", "Content-Type, Authorization"))
                .json(json)
        },
        Err(e) => {
            let err_msg = format!("Error uploading file: {}", e);
            println!("[ERROR] {}", err_msg);
            HttpResponse::InternalServerError()
                .append_header(("Access-Control-Allow-Origin", "*"))
                .append_header(("Access-Control-Allow-Methods", "POST, OPTIONS"))
                .append_header(("Access-Control-Allow-Headers", "Content-Type, Authorization"))
                .json(serde_json::json!({
                    "error": err_msg
                }))
        }
    }
}

#[get("/proxy/drive/{file_id}")]
pub async fn proxy_drive_file(path: web::Path<String>) -> Result<HttpResponse, Error> {
    let file_id = path.into_inner();
    println!("[DEBUG] proxy_drive_file: Menerima request proxy untuk file ID: {}", file_id);
    
    // Buat URL Google Drive
    let url = format!("https://drive.usercontent.google.com/download?id={}&export=view", file_id);
    
    // Buat HTTP client
    let client = Client::new();
    
    // Kirim request ke Google Drive
    match client.get(&url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .send()
        .await {
            Ok(response) => {
                // Dapatkan status dan headers
                let status = response.status();
                
                // Clone content-type header untuk digunakan nanti
                let content_type = response.headers()
                    .get("content-type")
                    .and_then(|h| h.to_str().ok())
                    .unwrap_or("application/octet-stream")
                    .to_string(); // Konversi ke String agar tidak terikat ke lifetime response
                
                // Log response
                println!("[DEBUG] proxy_drive_file: Response status: {}", status);
                println!("[DEBUG] proxy_drive_file: Content-Type: {}", content_type);
                
                // Jika response berhasil, teruskan ke client
                if status.is_success() {
                    // Dapatkan bytes dari response
                    match response.bytes().await {
                        Ok(bytes) => {
                            // Buat response dengan header yang sesuai
                            let http_response = HttpResponse::Ok()
                                .content_type(content_type)
                                .append_header(("Cache-Control", "public, max-age=86400"))
                                .append_header(("Access-Control-Allow-Origin", "*"))
                                .body(bytes);
                            
                            Ok(http_response)
                        },
                        Err(e) => {
                            println!("[ERROR] proxy_drive_file: Gagal membaca bytes: {}", e);
                            Ok(HttpResponse::InternalServerError().body(format!("Gagal membaca response: {}", e)))
                        }
                    }
                } else {
                    // Jika response gagal, kembalikan error
                    println!("[ERROR] proxy_drive_file: Google Drive mengembalikan status error: {}", status);
                    Ok(HttpResponse::build(status).body(format!("Google Drive error: {}", status)))
                }
            },
            Err(e) => {
                println!("[ERROR] proxy_drive_file: Gagal mengirim request ke Google Drive: {}", e);
                Ok(HttpResponse::InternalServerError().body(format!("Gagal mengirim request ke Google Drive: {}", e)))
            }
        }
}

pub fn upload_config(cfg: &mut web::ServiceConfig) {
    cfg.service(upload_image);
    cfg.service(proxy_drive_file);
}
