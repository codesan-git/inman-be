use actix_web::{post, get, patch, web, Responder, HttpResponse, Error, HttpRequest};
use crate::middleware::jwt_extractor::Claims;
use crate::services::drive_storage::{upload_file_handler, upload_file_with_item_id, DriveConfig, DriveClient};
use actix_multipart::Multipart;
use std::sync::Arc;
use tokio::sync::Mutex;
use reqwest::Client;
use sqlx::PgPool;
use uuid::Uuid;
use crate::routes::items::{UpdateItem, Item};

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

#[patch("/{id}/upload-image")]
pub async fn upload_item_image(
    claims: Claims, // Hapus underscore agar bisa digunakan untuk log
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
    payload: Multipart,
    config: web::Data<DriveConfig>,
    client: web::Data<Arc<Mutex<DriveClient>>>,
) -> impl Responder {
    let item_id = path.into_inner();
    println!("[INFO] upload_item_image: Uploading image for item ID: {}", item_id);
    
    // Ambil data item sebelum update untuk log
    let before_item = match sqlx::query_as::<_, Item>(
        "SELECT id, name, category_id, quantity, condition_id, location_id, photo_url, source_id, donor_id, procurement_id, status_id, value, created_at FROM items WHERE id = $1"
    )
        .bind(item_id)
        .fetch_optional(pool.get_ref())
        .await {
            Ok(Some(item)) => {
                println!("[DEBUG] Item sebelum update: ID: {}, photo_url: {:?}", item.id, item.photo_url);
                Some(item)
            },
            Ok(None) => return HttpResponse::NotFound().json(serde_json::json!({"error": "Item not found"})),
            Err(e) => return HttpResponse::InternalServerError().json(serde_json::json!({"error": format!("Failed to fetch item: {}", e)}))
        };
    
    // Mulai transaksi
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => return HttpResponse::InternalServerError().json(serde_json::json!({"error": format!("Failed to start transaction: {}", e)}))
    };
    
    // Gunakan fungsi upload_file_with_item_id yang menerima item_id
    match upload_file_with_item_id(payload, config, client, item_id).await {
        Ok(file_url) => {
            // file_url adalah String URL langsung, bukan JSON
            let photo_url = file_url;
            println!("[DEBUG] New photo_url: {}", photo_url);
            
            // Update item photo_url in DB
            let update = UpdateItem {
                name: None,
                category_id: None,
                quantity: None,
                condition_id: None,
                location_id: None,
                photo_url: Some(photo_url),
                source_id: None,
                donor_id: None,
                procurement_id: None,
                status_id: None,
                value: None, // Nilai barang (opsional)
            };
            
            let updated_item = sqlx::query_as::<_, Item>(
                "UPDATE items SET photo_url = $1 WHERE id = $2 RETURNING *"
            )
            .bind(&update.photo_url)
            .bind(item_id)
            .fetch_optional(&mut *tx)
            .await;
            
            match updated_item {
                Ok(Some(item)) => {
                    // Buat log dengan data before dan after yang lengkap
                    println!("[DEBUG] Item setelah update: ID: {}, photo_url: {:?}", item.id, item.photo_url);
                    
                    let before_json = before_item.map(|b| serde_json::to_value(&b).unwrap());
                    let after_json = serde_json::to_value(&item).unwrap();
                    
                    println!("[DEBUG] Before JSON: {:?}", before_json);
                    println!("[DEBUG] After JSON: {:?}", after_json);
                    
                    let log_result = sqlx::query(
                        "INSERT INTO item_logs (item_id, action, before, after, by) VALUES ($1, $2, $3, $4, $5)"
                    )
                    .bind(item_id)
                    .bind("update")
                    .bind(before_json)
                    .bind(Some(after_json))
                    .bind(uuid::Uuid::parse_str(&claims.sub).ok())
                    .execute(&mut *tx)
                    .await;
                    
                    if let Err(e) = log_result {
                        let _ = tx.rollback().await;
                        return HttpResponse::InternalServerError().json(serde_json::json!({"error": format!("Failed to create log: {}", e)}));
                    }
                    
                    // Commit transaksi
                    match tx.commit().await {
                        Ok(_) => HttpResponse::Ok().json(item),
                        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": format!("Failed to commit transaction: {}", e)}))
                    }
                },
                Ok(None) => {
                    let _ = tx.rollback().await;
                    HttpResponse::NotFound().json(serde_json::json!({"error": "Item not found"}))
                },
                Err(e) => {
                    let _ = tx.rollback().await;
                    HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()}))
                },
            }
        },
        Err(e) => {
            HttpResponse::InternalServerError().json(serde_json::json!({"error": format!("Upload failed: {}", e)}))
        }
    }
}

use futures::TryStreamExt;

#[patch("/{id}/update-with-image")]
pub async fn update_item_with_image(
    claims: Claims,
    pool: web::Data<PgPool>,
    path: web::Path<Uuid>,
    mut payload: Multipart,
    config: web::Data<DriveConfig>,
    client: web::Data<Arc<Mutex<DriveClient>>>,
) -> impl Responder {
    let item_id = path.into_inner();
    println!("[INFO] update_item_with_image: Updating item and uploading image for item ID: {}", item_id);
    
    // Ambil data sebelum update dan pastikan tidak null
    let before = match sqlx::query_as::<_, Item>(
        "SELECT id, name, category_id, quantity, condition_id, location_id, photo_url, source_id, donor_id, procurement_id, status_id, value, created_at FROM items WHERE id = $1"
    )
        .bind(item_id)
        .fetch_optional(pool.get_ref())
        .await {
            Ok(Some(item)) => {
                println!("[DEBUG] Item ditemukan dengan ID: {}, photo_url: {:?}", item.id, item.photo_url);
                Some(item)
            },
            Ok(None) => return HttpResponse::NotFound().json(serde_json::json!({"error": "Item not found"})),
            Err(e) => return HttpResponse::InternalServerError().json(serde_json::json!({"error": format!("Failed to fetch item: {}", e)}))
        };
    
    // Ekstrak file dan data item dari payload
    let mut file_data: Option<(String, Vec<u8>, String)> = None;
    let mut item_data_json: Option<String> = None;
    let mut explicit_content_type: Option<String> = None;
    
    // Proses payload multipart untuk mendapatkan file dan data item
    while let Some(mut field) = match payload.try_next().await {
        Ok(field) => field,
        Err(e) => {
            println!("[ERROR] Error extracting field from multipart: {}", e);
            return HttpResponse::BadRequest().json(serde_json::json!({"error": format!("Error extracting field: {}", e)}));
        }
    } {
        let content_disposition = field.content_disposition().clone();
        let field_name = content_disposition.get_name().unwrap_or("");
        
        println!("[DEBUG] Processing field: {}", field_name);
        
        if field_name == "file" {
            // Ekstrak file
            let filename = content_disposition
                .get_filename()
                .map(|f| sanitize_filename::sanitize(f))
                .unwrap_or_else(|| "unknown_file".to_string());
            
            let content_type = field.content_type().clone()
                .map(|t| t.to_string())
                .unwrap_or_else(|| "application/octet-stream".to_string());
            
            let mut bytes = Vec::new();
            while let Some(chunk) = match field.try_next().await {
                Ok(chunk) => chunk,
                Err(e) => {
                    println!("[ERROR] Error reading file chunk: {}", e);
                    return HttpResponse::InternalServerError().json(serde_json::json!({"error": format!("Error reading file: {}", e)}));
                }
            } {
                bytes.extend_from_slice(&chunk);
            }
            
            println!("[DEBUG] File extracted: {} ({} bytes)", &filename, bytes.len());
            file_data = Some((filename, bytes, content_type));
        } else if field_name == "itemData" {
            // Ekstrak data item
            let mut data = String::new();
            while let Some(chunk) = match field.try_next().await {
                Ok(chunk) => chunk,
                Err(e) => {
                    println!("[ERROR] Error reading itemData chunk: {}", e);
                    return HttpResponse::InternalServerError().json(serde_json::json!({"error": format!("Error reading itemData: {}", e)}));
                }
            } {
                data.push_str(std::str::from_utf8(&chunk).unwrap_or(""));
            }
            
            println!("[DEBUG] Item data extracted: {}", &data);
            item_data_json = Some(data);
        } else if field_name == "contentType" {
            // Ekstrak content type yang dikirim dari frontend
            let mut content_type = String::new();
            while let Some(chunk) = match field.try_next().await {
                Ok(chunk) => chunk,
                Err(e) => {
                    println!("[ERROR] Error reading contentType chunk: {}", e);
                    return HttpResponse::InternalServerError().json(serde_json::json!({"error": format!("Error reading contentType: {}", e)}));
                }
            } {
                content_type.push_str(std::str::from_utf8(&chunk).unwrap_or(""));
            }
            
            println!("[DEBUG] Explicit content type received: {}", &content_type);
            explicit_content_type = Some(content_type);
        }
    }
    
    // Mulai transaksi
    let mut tx = match pool.begin().await {
        Ok(tx) => tx,
        Err(e) => return HttpResponse::InternalServerError().json(serde_json::json!({"error": format!("Failed to start transaction: {}", e)}))
    };
    
    // Parse item data jika ada
    let mut update = UpdateItem {
        name: None,
        category_id: None,
        quantity: None,
        condition_id: None,
        location_id: None,
        photo_url: None,
        source_id: None,
        donor_id: None,
        procurement_id: None,
        status_id: None,
        value: None, // Nilai barang (opsional)
    };
    
    if let Some(data_json) = item_data_json {
        match serde_json::from_str::<UpdateItem>(&data_json) {
            Ok(parsed_data) => {
                update = parsed_data;
                println!("[DEBUG] Successfully parsed item data: {:?}", update);
            },
            Err(e) => {
                println!("[ERROR] Failed to parse item data: {}", e);
                // Continue with empty update data, we'll still update the photo_url
            }
        }
    }
    
    // Upload file ke Google Drive jika ada
    if let Some((filename, bytes, content_type)) = file_data {
        // Use explicit content type if available, otherwise use the one from the file
        let final_content_type = explicit_content_type.unwrap_or(content_type);
        println!("[DEBUG] Using content type for upload: {}", final_content_type);
        
        match crate::services::drive_storage::upload_file_with_item_id_field(filename, bytes, final_content_type, config, client, item_id).await {
            Ok(url) => {
                update.photo_url = Some(url);
                println!("[DEBUG] File uploaded successfully, URL: {}", update.photo_url.as_ref().unwrap());
            },
            Err(e) => {
                let _ = tx.rollback().await;
                return HttpResponse::InternalServerError().json(serde_json::json!({"error": format!("Upload failed: {}", e)}));
            }
        }
    }
    
    // Buat query dinamis berdasarkan field yang diupdate
    let mut param_index = 1;
    
    // Buat query builder
    let mut query_builder = sqlx::query_as::<_, Item>(
        "UPDATE items SET "
    );
    
    // Gunakan pendekatan yang lebih sederhana untuk query dinamis
    let mut set_clauses = Vec::new();
    
    if let Some(name) = &update.name {
        set_clauses.push(format!("name = ${}", param_index));
        query_builder = query_builder.bind(name);
        param_index += 1;
    }
    
    if let Some(category_id) = &update.category_id {
        set_clauses.push(format!("category_id = ${}", param_index));
        query_builder = query_builder.bind(category_id);
        param_index += 1;
    }
    
    if let Some(quantity) = &update.quantity {
        set_clauses.push(format!("quantity = ${}", param_index));
        query_builder = query_builder.bind(quantity);
        param_index += 1;
    }
    
    if let Some(condition_id) = &update.condition_id {
        set_clauses.push(format!("condition_id = ${}", param_index));
        query_builder = query_builder.bind(condition_id);
        param_index += 1;
    }
    
    if let Some(location_id) = &update.location_id {
        set_clauses.push(format!("location_id = ${}", param_index));
        query_builder = query_builder.bind(location_id);
        param_index += 1;
    }
    
    if let Some(photo_url) = &update.photo_url {
        set_clauses.push(format!("photo_url = ${}", param_index));
        query_builder = query_builder.bind(photo_url);
        param_index += 1;
    }
    
    if let Some(source_id) = &update.source_id {
        set_clauses.push(format!("source_id = ${}", param_index));
        query_builder = query_builder.bind(source_id);
        param_index += 1;
    }
    
    if let Some(donor_id) = &update.donor_id {
        set_clauses.push(format!("donor_id = ${}", param_index));
        query_builder = query_builder.bind(donor_id);
        param_index += 1;
    }
    
    if let Some(procurement_id) = &update.procurement_id {
        set_clauses.push(format!("procurement_id = ${}", param_index));
        query_builder = query_builder.bind(procurement_id);
        param_index += 1;
    }
    
    if let Some(status_id) = &update.status_id {
        set_clauses.push(format!("status_id = ${}", param_index));
        query_builder = query_builder.bind(status_id);
        param_index += 1;
    }
    
    // Jika tidak ada field yang diupdate, return error
    if set_clauses.is_empty() {
        let _ = tx.rollback().await;
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "No fields to update"}));
    }
    
    // Buat query lengkap
    let full_query = format!(
        "UPDATE items SET {} WHERE id = ${} RETURNING *",
        set_clauses.join(", "),
        param_index
    );
    
    println!("[DEBUG] Update query: {}", full_query);
    
    // Buat query builder baru dengan query lengkap
    let mut query_builder = sqlx::query_as::<_, Item>(&full_query);
    
    // Bind semua parameter
    if let Some(name) = &update.name {
        query_builder = query_builder.bind(name);
    }
    
    if let Some(category_id) = &update.category_id {
        query_builder = query_builder.bind(category_id);
    }
    
    if let Some(quantity) = &update.quantity {
        query_builder = query_builder.bind(quantity);
    }
    
    if let Some(condition_id) = &update.condition_id {
        query_builder = query_builder.bind(condition_id);
    }
    
    if let Some(location_id) = &update.location_id {
        query_builder = query_builder.bind(location_id);
    }
    
    if let Some(photo_url) = &update.photo_url {
        query_builder = query_builder.bind(photo_url);
    }
    
    if let Some(source_id) = &update.source_id {
        query_builder = query_builder.bind(source_id);
    }
    
    if let Some(donor_id) = &update.donor_id {
        query_builder = query_builder.bind(donor_id);
    }
    
    if let Some(procurement_id) = &update.procurement_id {
        query_builder = query_builder.bind(procurement_id);
    }
    
    if let Some(status_id) = &update.status_id {
        query_builder = query_builder.bind(status_id);
    }
    
    // Bind item_id
    query_builder = query_builder.bind(item_id);
    
    // Execute query
    let updated_item = query_builder.fetch_optional(&mut *tx).await;
    
    match updated_item {
        Ok(Some(item)) => {
            // Debug: Cetak nilai before untuk debugging
            println!("[DEBUG] Before value: {:?}", before);
            let before_json = before.map(|b| serde_json::to_value(&b).unwrap());
            println!("[DEBUG] Before JSON: {:?}", before_json);
            
            // Debug: Cetak nilai after untuk debugging
            println!("[DEBUG] After value: {:?}", item);
            let after_json = serde_json::to_value(&item).unwrap();
            println!("[DEBUG] After JSON: {:?}", after_json);
            
            // Buat log setelah kedua proses berhasil
            let log_result = sqlx::query(
                "INSERT INTO item_logs (item_id, action, before, after, by) VALUES ($1, $2, $3, $4, $5)"
            )
            .bind(item_id)
            .bind("update")
            .bind(before_json)
            .bind(Some(after_json))
            .bind(uuid::Uuid::parse_str(&claims.sub).ok())
            .execute(&mut *tx)
            .await;
            
            if let Err(e) = log_result {
                let _ = tx.rollback().await;
                return HttpResponse::InternalServerError().json(serde_json::json!({"error": format!("Failed to create log: {}", e)}));
            }
            
            // Commit transaksi
            match tx.commit().await {
                Ok(_) => HttpResponse::Ok().json(item),
                Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({"error": format!("Failed to commit transaction: {}", e)}))
            }
        },
        Ok(None) => {
            let _ = tx.rollback().await;
            HttpResponse::NotFound().json(serde_json::json!({"error": "Item not found"}))
        },
        Err(e) => {
            let _ = tx.rollback().await;
            HttpResponse::InternalServerError().json(serde_json::json!({"error": e.to_string()}))
        },
    }
}

pub fn upload_config(cfg: &mut web::ServiceConfig) {
    cfg.service(upload_image);
    cfg.service(proxy_drive_file);
    cfg.service(upload_item_image);
    cfg.service(update_item_with_image);
}
