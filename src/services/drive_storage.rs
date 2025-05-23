use actix_multipart::Multipart;
use actix_web::{web, Error};
use futures::{StreamExt, TryStreamExt};
use sanitize_filename::sanitize;
use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;
use std::path::Path;
use mime::Mime;
use reqwest::{Client, multipart};
use serde::{Deserialize, Serialize};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use chrono::{Duration, Utc};

/// Konfigurasi untuk Google Drive storage
#[derive(Clone, Debug)]
pub struct DriveConfig {
    pub credentials_json: String,
    pub folder_id: String,
    pub max_file_size: usize,
    pub allowed_types: Vec<String>,
    pub public_base_url: String,
    pub base_url: String,
}

impl Default for DriveConfig {
    fn default() -> Self {
        Self {
            credentials_json: std::env::var("GOOGLE_CREDENTIALS_JSON")
                .unwrap_or_else(|_| "{}".to_string()),
            folder_id: std::env::var("GOOGLE_DRIVE_FOLDER_ID")
                .unwrap_or_else(|_| "".to_string()),
            max_file_size: std::env::var("MAX_FILE_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5_000_000), // 5MB default
            allowed_types: vec![
                "image/jpeg".to_string(),
                "image/png".to_string(),
                "image/webp".to_string(),
                "image/gif".to_string(),
            ],
            public_base_url: std::env::var("GOOGLE_DRIVE_PUBLIC_URL")
                .unwrap_or_else(|_| "https://drive.google.com/uc?export=view&id=".to_string()),
            base_url: std::env::var("BASE_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
        }
    }
}

/// Struktur untuk menyimpan Google Drive client
pub struct DriveClient {
    pub client: Client,
    pub credentials: GoogleCredentials,
    pub access_token: Option<String>,
    pub token_expiry: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct GoogleCredentials {
    pub r#type: String,
    pub project_id: String,
    pub private_key_id: String,
    pub private_key: String,
    pub client_email: String,
    pub client_id: String,
    pub auth_uri: String,
    pub token_uri: String,
    pub auth_provider_x509_cert_url: String,
    pub client_x509_cert_url: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: i64,
}

/// Membuat Google Drive client
pub async fn create_drive_client(config: &DriveConfig) -> Result<DriveClient, Box<dyn std::error::Error>> {
    // Parse credentials JSON
    let credentials_json = &config.credentials_json;
    
    // Log untuk debugging (hanya 50 karakter pertama untuk keamanan)
    println!("[DEBUG] Credentials JSON format: {}", 
             credentials_json.chars().take(50).collect::<String>());
    
    let credentials: GoogleCredentials = match serde_json::from_str(credentials_json) {
        Ok(creds) => creds,
        Err(e) => {
            println!("[ERROR] Gagal parsing credentials JSON: {}", e);
            return Err(format!("Failed to parse credentials JSON: {}", e).into());
        }
    };
    
    // Buat HTTP client
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()?;
    
    let drive_client = DriveClient {
        client,
        credentials,
        access_token: None,
        token_expiry: None,
    };
    
    Ok(drive_client)
}

/// Mendapatkan token akses untuk Google Drive API
async fn get_access_token(client: &mut DriveClient) -> Result<String, Box<dyn std::error::Error>> {
    // Cek apakah token masih valid
    if let (Some(token), Some(expiry)) = (&client.access_token, client.token_expiry) {
        if expiry > Utc::now() + Duration::minutes(5) {
            return Ok(token.clone());
        }
    }
    
    // Buat JWT claim
    let now = Utc::now();
    let exp = now + Duration::minutes(60);
    
    #[derive(Debug, Serialize)]
    struct Claims {
        iss: String,
        scope: String,
        aud: String,
        exp: i64,
        iat: i64,
    }
    
    let claims = Claims {
        iss: client.credentials.client_email.clone(),
        scope: "https://www.googleapis.com/auth/drive https://www.googleapis.com/auth/drive.file".to_string(),
        aud: "https://oauth2.googleapis.com/token".to_string(),
        exp: exp.timestamp(),
        iat: now.timestamp(),
    };
    
    // Buat header JWT
    let header = Header::new(Algorithm::RS256);
    
    // Gunakan fungsi utilitas untuk memperbaiki format private key
    let fixed_private_key = fix_private_key_format(&client.credentials.private_key);
    
    // Log untuk debugging (hanya tampilkan sebagian untuk keamanan)
    println!("[DEBUG] Private key format (first 50 chars): {}", fixed_private_key.chars().take(50).collect::<String>());
    println!("[DEBUG] Private key format (last 50 chars): {}", fixed_private_key.chars().rev().take(50).collect::<String>());
    
    // Coba beberapa format encoding key dengan fallback
    let private_key = match EncodingKey::from_rsa_pem(fixed_private_key.as_bytes()) {
        Ok(key) => {
            println!("[INFO] Berhasil menggunakan format PEM standar");
            key
        },
        Err(e) => {
            println!("[WARN] Format PEM standar gagal: {}", e);
            
            // Fallback 1: Coba format RSA private key
            let raw_key = client.credentials.private_key.replace("\\n", "\n")
                .replace("-----BEGIN PRIVATE KEY-----", "")
                .replace("-----END PRIVATE KEY-----", "")
                .replace("\n", "");
                
            let rsa_key = format!("-----BEGIN RSA PRIVATE KEY-----\n{}\n-----END RSA PRIVATE KEY-----", raw_key);
            
            match EncodingKey::from_rsa_pem(rsa_key.as_bytes()) {
                Ok(key) => {
                    println!("[INFO] Berhasil menggunakan format RSA PRIVATE KEY");
                    key
                },
                Err(e2) => {
                    println!("[WARN] Format RSA private key gagal: {}", e2);
                    
                    // Fallback 2: Coba dengan encoding PKCS8 dengan base64 decode
                    let pkcs8_key = format!("-----BEGIN PRIVATE KEY-----\n{}\n-----END PRIVATE KEY-----", raw_key);
                    match EncodingKey::from_rsa_pem(pkcs8_key.as_bytes()) {
                        Ok(key) => {
                            println!("[INFO] Berhasil menggunakan format PKCS8");
                            key
                        },
                        Err(e3) => {
                            // Fallback 3: Coba format lain jika semua gagal
                            println!("[ERROR] Semua format private key gagal: {}, {}, {}", e, e2, e3);
                            return Err(format!("InvalidKeyFormat: Tidak dapat memproses format private key").into());
                        }
                    }
                }
            }
        }
    };
    
    // Encode JWT dengan jsonwebtoken
    let jwt = encode(&header, &claims, &private_key)?;
    
    // Request token
    let params = [
        ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
        ("assertion", &jwt)
    ];
    
    let response = client.client.post("https://oauth2.googleapis.com/token")
        .form(&params)
        .send()
        .await?;
    
    if !response.status().is_success() {
        return Err(format!("Failed to get token: {}", response.text().await?).into());
    }
    
    let token_response: TokenResponse = response.json().await?;
    
    // Simpan token dan waktu kedaluwarsa
    client.access_token = Some(token_response.access_token.clone());
    client.token_expiry = Some(Utc::now() + Duration::seconds(token_response.expires_in));
    
    Ok(token_response.access_token)
}

/// Pastikan folder untuk upload sudah ada
pub async fn ensure_folder_exists(client: &mut DriveClient, folder_id: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Dapatkan token akses
    let token = get_access_token(client).await?;
    
    // Jika folder ID sudah ada, gunakan itu
    if !folder_id.is_empty() {
        // Verifikasi folder ID
        let response = client.client.get(&format!("https://www.googleapis.com/drive/v3/files/{}", folder_id))
            .query(&[("fields", "id,name,mimeType")])
            .bearer_auth(&token)
            .send()
            .await?;
            
        if response.status().is_success() {
            let file: serde_json::Value = response.json().await?;
            if file["mimeType"] == "application/vnd.google-apps.folder" {
                return Ok(folder_id.to_string());
            } else {
                return Err("ID yang disediakan bukan folder".into());
            }
        } else {
            return Err(format!("Gagal memverifikasi folder: {}", response.text().await?).into());
        }
    }
    
    // Buat folder baru jika tidak ada folder ID
    let folder_metadata = serde_json::json!({
        "name": "actisol-uploads",
        "mimeType": "application/vnd.google-apps.folder"
    });
    
    // Buat folder
    let response = client.client.post("https://www.googleapis.com/drive/v3/files")
        .query(&[("fields", "id")])
        .bearer_auth(&token)
        .json(&folder_metadata)
        .send()
        .await?;
        
    if !response.status().is_success() {
        return Err(format!("Gagal membuat folder: {}", response.text().await?).into());
    }
    
    let folder_response: serde_json::Value = response.json().await?;
    let new_folder_id = folder_response["id"].as_str().ok_or("Gagal mendapatkan ID folder")?;
    
    // Atur permission agar folder bisa diakses publik
    let permission = serde_json::json!({
        "role": "reader",
        "type": "anyone"
    });
    
    let perm_response = client.client.post(&format!("https://www.googleapis.com/drive/v3/files/{}/permissions", new_folder_id))
        .bearer_auth(&token)
        .json(&permission)
        .send()
        .await?;
        
    if !perm_response.status().is_success() {
        return Err(format!("Gagal mengatur permission: {}", perm_response.text().await?).into());
    }
    
    Ok(new_folder_id.to_string())
}

/// Upload file ke Google Drive
pub async fn upload_to_drive(
    client: &mut DriveClient,
    folder_id: &str,
    filename: &str,
    data: Vec<u8>,
    content_type: Option<Mime>,
) -> Result<String, Box<dyn std::error::Error>> {
    println!("[DEBUG] Memulai upload file ke Google Drive: {}", filename);
    println!("[DEBUG] Ukuran file: {} bytes", data.len());
    println!("[DEBUG] Folder ID: {}", folder_id);
    println!("[DEBUG] Content-Type: {:?}", content_type);
    
    // Dapatkan token akses
    println!("[DEBUG] Mendapatkan access token...");
    let token = match get_access_token(client).await {
        Ok(t) => {
            println!("[DEBUG] Token berhasil didapatkan");
            t
        },
        Err(e) => {
            println!("[ERROR] Gagal mendapatkan token: {}", e);
            return Err(e);
        }
    };
    
    // Tentukan MIME type yang valid untuk Google Drive
    // Pastikan kita menggunakan tipe konten yang didukung oleh Google Drive
    let mime_str = match content_type {
        Some(mime) => {
            // Jika tipe konten adalah image, gunakan tipe konten yang sesuai
            if mime.type_() == mime::IMAGE {
                mime.to_string()
            } else {
                // Default ke image/jpeg jika bukan tipe image yang dikenal
                "image/jpeg".to_string()
            }
        },
        None => "image/jpeg".to_string() // Default ke image/jpeg
    };
    println!("[DEBUG] MIME type: {}", mime_str);
    
    // Buat metadata file
    let metadata = serde_json::json!({
        "name": filename,
        "parents": [folder_id],
        "mimeType": mime_str
    });
    println!("[DEBUG] Metadata: {}", metadata);
    
    // Buat form multipart dengan boundary yang eksplisit
    println!("[DEBUG] Membuat form multipart...");
    let form = multipart::Form::new()
        // Metadata file dalam format JSON dengan Content-Type yang benar
        .part("metadata", multipart::Part::text(metadata.to_string())
            .mime_str("application/json")?)
        // Konten file dengan Content-Type yang benar
        .part("file", multipart::Part::bytes(data)
            .file_name(filename.to_string())
            .mime_str(&mime_str)?
        );
    
    // Upload file
    println!("[DEBUG] Mengirim request upload ke Google Drive API...");
    let response = match client.client.post("https://www.googleapis.com/upload/drive/v3/files?uploadType=multipart")
        .query(&[("fields", "id")])
        .bearer_auth(&token)
        .multipart(form)
        .send()
        .await {
            Ok(resp) => resp,
            Err(e) => {
                println!("[ERROR] Gagal mengirim request: {}", e);
                return Err(Box::new(e));
            }
        };
    
    println!("[DEBUG] Response status: {}", response.status());
    
    if !response.status().is_success() {
        let error_text = response.text().await?;
        println!("[ERROR] Gagal upload file: {}", error_text);
        return Err(format!("Gagal upload file: {}", error_text).into());
    }
    
    // Dapatkan ID file
    let file_response_text = match response.text().await {
        Ok(text) => {
            println!("[DEBUG] Response body: {}", text);
            text
        },
        Err(e) => {
            println!("[ERROR] Gagal membaca response body: {}", e);
            return Err(Box::new(e));
        }
    };
    
    let file_response: serde_json::Value = match serde_json::from_str(&file_response_text) {
        Ok(json) => json,
        Err(e) => {
            println!("[ERROR] Gagal parsing JSON response: {}", e);
            return Err(Box::new(e));
        }
    };
    
    let file_id = match file_response.get("id").and_then(|v| v.as_str()) {
        Some(id) => {
            println!("[DEBUG] File ID: {}", id);
            id
        },
        None => {
            println!("[ERROR] Gagal mendapatkan ID file dari response");
            return Err("Gagal mendapatkan ID file".into());
        }
    };
    
    // Atur permission agar file bisa diakses publik
    println!("[DEBUG] Mengatur permission publik...");
    
    // Permission untuk publik (anyone)
    let permission_public = serde_json::json!({
        "role": "reader",
        "type": "anyone",
        "allowFileDiscovery": true,
        "published": true
    });
    
    // Tambahkan parameter untuk permission
    let perm_url = format!("https://www.googleapis.com/drive/v3/files/{}/permissions?supportsAllDrives=true&fields=id", file_id);
    println!("[DEBUG] URL permission: {}", perm_url);
    
    let perm_response = match client.client.post(&perm_url)
        .bearer_auth(&token)
        .header("Content-Type", "application/json")
        .json(&permission_public)
        .send()
        .await {
            Ok(resp) => resp,
            Err(e) => {
                println!("[ERROR] Gagal mengirim request permission: {}", e);
                return Err(Box::new(e));
            }
        };
    
    println!("[DEBUG] Permission response status: {}", perm_response.status());
    
    if !perm_response.status().is_success() {
        let error_text = perm_response.text().await?;
        println!("[ERROR] Gagal mengatur permission: {}", error_text);
        return Err(format!("Gagal mengatur permission: {}", error_text).into());
    }
    
    // Ubah pengaturan file untuk memastikan file dapat diakses publik
    println!("[DEBUG] Mengubah pengaturan file untuk akses publik...");
    let update_file_url = format!("https://www.googleapis.com/drive/v3/files/{}?supportsAllDrives=true&fields=webContentLink,webViewLink", file_id);
    
    let update_file_data = serde_json::json!({
        "copyRequiresWriterPermission": false,
        "viewersCanCopyContent": true,
        "writersCanShare": true,
        "published": true,
        "publishedOutsideDomain": true,
        "publiclyViewable": true
    });
    
    let update_response_result = client.client.patch(&update_file_url)
        .bearer_auth(&token)
        .header("Content-Type", "application/json")
        .json(&update_file_data)
        .send()
        .await;
        
    if let Ok(update_response) = update_response_result {
        println!("[DEBUG] Update file response status: {}", update_response.status());
        
        // Jika update berhasil, coba dapatkan webContentLink dan webViewLink
        if update_response.status().is_success() {
            if let Ok(response_json) = update_response.json::<serde_json::Value>().await {
                if let Some(web_content_link) = response_json.get("webContentLink").and_then(|v| v.as_str()) {
                    println!("[DEBUG] webContentLink: {}", web_content_link);
                }
                if let Some(web_view_link) = response_json.get("webViewLink").and_then(|v| v.as_str()) {
                    println!("[DEBUG] webViewLink: {}", web_view_link);
                }
            }
        } else {
            if let Ok(error_text) = update_response.text().await {
                println!("[WARN] Gagal mengubah pengaturan file: {}", error_text);
            }
        }
    } else if let Err(e) = update_response_result {
        println!("[ERROR] Gagal mengirim request update file: {}", e);
    }
    
    // Kode untuk mengecek respons update file sudah ditangani di atas
    
    // Kode untuk mengecek respons update file sudah ditangani di atas
    
    println!("[DEBUG] Upload berhasil, file ID: {}", file_id);
    
    // Kembalikan file ID saja, URL akan dibuat di fungsi pemanggil
    println!("[DEBUG] Mengembalikan file ID: {}", file_id);
    
    Ok(file_id.to_string())
}

/// Menangani upload file gambar
// Fungsi upload_image telah dihapus karena tidak digunakan lagi
// Digantikan oleh fungsi upload_file_handler dan upload_to_drive_or_local

/// Fungsi untuk upload file ke Google Drive, mengembalikan error jika gagal tanpa fallback ke penyimpanan lokal
pub async fn upload_to_drive_or_local(
    mut payload: Multipart,
    config: web::Data<DriveConfig>,
    client: web::Data<Arc<Mutex<DriveClient>>>,
) -> Result<String, Box<dyn std::error::Error>> {
    println!("[DEBUG] upload_to_drive_or_local: Memulai proses upload");
    
    // Proses file upload
    while let Ok(Some(mut field)) = payload.try_next().await {
        // Dapatkan content disposition
        let content_disposition = field.content_disposition();
        
        // Dapatkan nama file
        let filename = content_disposition
            .get_filename()
            .map_or_else(|| Uuid::new_v4().to_string(), sanitize);
        
        // Buat nama file unik dengan timestamp dan UUID
        let uuid = Uuid::new_v4();
        let timestamp = chrono::Utc::now().timestamp();
        let file_ext = Path::new(&filename)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("jpg");
        
        let unique_filename = format!("{}-{}.{}", timestamp, uuid, file_ext);
        println!("[DEBUG] upload_to_drive_or_local: Filename: {}", unique_filename);
        
        // Dapatkan content type
        let content_type_opt = field.content_type().map(|ct| ct.clone());
        println!("[DEBUG] upload_to_drive_or_local: Content type: {:?}", content_type_opt);
        
        // Validasi tipe file
        if let Some(content_type) = &content_type_opt {
            if !config.allowed_types.contains(&content_type.to_string()) {
                println!("[WARN] upload_to_drive_or_local: Tipe file tidak diizinkan: {}", content_type);
                return Err(format!("Tipe file tidak diizinkan: {}", content_type).into());
            }
        }
        
        // Baca data file
        let mut data = Vec::new();
        while let Some(chunk) = field.next().await {
            let chunk_data = chunk?;
            data.extend_from_slice(&chunk_data);
            
            // Cek ukuran file
            if data.len() > config.max_file_size {
                println!("[WARN] upload_to_drive_or_local: Ukuran file melebihi batas: {} > {}", data.len(), config.max_file_size);
                return Err(format!("Ukuran file melebihi batas: {} > {}", data.len(), config.max_file_size).into());
            }
        }
        
        println!("[DEBUG] upload_to_drive_or_local: File size: {} bytes", data.len());
        
        // Prioritaskan upload ke Google Drive terlebih dahulu
        println!("[INFO] upload_to_drive_or_local: Mencoba upload ke Google Drive");
        match try_upload_to_google_drive(
            client.clone(), 
            &config, 
            &unique_filename, 
            data.clone(), 
            content_type_opt.clone()
        ).await {
            Ok(drive_url) => {
                println!("[INFO] upload_to_drive_or_local: Berhasil upload ke Google Drive: {}", drive_url);
                
                // Kembalikan URL Google Drive yang sudah berhasil
                return Ok(drive_url);
            },
            Err(e) => {
                println!("[ERROR] upload_to_drive_or_local: Gagal upload ke Google Drive: {}", e);
                
                // Kembalikan error tanpa fallback ke penyimpanan lokal
                return Err(format!("Gagal upload file ke Google Drive: {}", e).into());
            }
        }
    }
    
    Err("Tidak ada file yang diupload".into())
}

/// Mencoba upload ke Google Drive
async fn try_upload_to_google_drive(
    client: web::Data<Arc<Mutex<DriveClient>>>,
    config: &web::Data<DriveConfig>,
    filename: &str,
    data: Vec<u8>,
    content_type: Option<Mime>,
) -> Result<String, Box<dyn std::error::Error>> {
    println!("[DEBUG] try_upload_to_google_drive: Mencoba upload ke Google Drive");
    
    // Lock client untuk operasi upload
    let mut client_guard = client.lock().await;
    
    // Upload ke Google Drive
    match upload_to_drive(
        &mut client_guard, 
        &config.folder_id, 
        filename, 
        data, 
        content_type
    ).await {
        Ok(file_id) => {
            // Gunakan format URL untuk akses langsung ke konten file
            let public_url = get_public_url(config, &file_id);
            println!("[DEBUG] URL publik yang dibuat: {}", public_url);
            Ok(public_url)
        },
        Err(e) => {
            Err(e)
        }
    }
}

/// Fungsi utilitas untuk memperbaiki format private key
fn fix_private_key_format(private_key: &str) -> String {
    let mut fixed_key = private_key.to_string();
    
    // Ganti escaped newlines dengan newline sebenarnya
    fixed_key = fixed_key.replace("\\n", "\n");
    
    // Pastikan format PEM yang benar
    if !fixed_key.contains("-----BEGIN") {
        // Jika tidak ada header PEM, tambahkan header dan footer
        fixed_key = format!("-----BEGIN PRIVATE KEY-----\n{}\n-----END PRIVATE KEY-----", fixed_key);
    }
    
    // Pastikan ada newline setelah header dan sebelum footer
    for header in ["-----BEGIN PRIVATE KEY-----", "-----BEGIN RSA PRIVATE KEY-----"] {
        if fixed_key.contains(header) && !fixed_key.contains(&format!("{}{}", header, "\n")) {
            fixed_key = fixed_key.replace(header, &format!("{}{}", header, "\n"));
        }
    }
    
    for footer in ["-----END PRIVATE KEY-----", "-----END RSA PRIVATE KEY-----"] {
        if fixed_key.contains(footer) && !fixed_key.contains(&format!("{}{}", "\n", footer)) {
            fixed_key = fixed_key.replace(footer, &format!("{}{}", "\n", footer));
        }
    }
    
    fixed_key
}

/// Mendapatkan URL publik untuk file Google Drive
pub fn get_public_url(config: &DriveConfig, file_id: &str) -> String {
    // Deteksi environment: jika BASE_URL adalah localhost tapi kita di production, gunakan URL production
    let base_url = if config.base_url.contains("localhost") {
        // Cek jika ada environment variable untuk production URL
        match std::env::var("PRODUCTION_URL") {
            Ok(url) if !url.is_empty() => url,
            _ => {
                // Jika tidak ada PRODUCTION_URL, gunakan render.com URL jika kita di render
                if std::env::var("RENDER").is_ok() {
                    "https://inman-be.onrender.com".to_string()
                } else {
                    config.base_url.clone()
                }
            }
        }
    } else {
        config.base_url.clone()
    };
    
    // Log URL yang digunakan
    println!("[INFO] Menggunakan base URL: {}", base_url);
    
    // Menggunakan endpoint proxy di backend dengan URL yang sudah dikoreksi
    format!("{}/api/upload/proxy/drive/{}", base_url, file_id)
}

/// Upload file ke penyimpanan lokal (fallback)
async fn upload_local(
    filename: &str,
    data: Vec<u8>,
    _content_type: Option<Mime>,
) -> Result<String, Box<dyn std::error::Error>> {
    println!("[DEBUG] upload_local: Mencoba upload ke penyimpanan lokal");
    
    // Buat direktori uploads jika belum ada
    let upload_dir = "uploads";
    if !Path::new(upload_dir).exists() {
        std::fs::create_dir_all(upload_dir)?;
    }
    
    // Simpan file
    let file_path = format!("{}/{}", upload_dir, filename);
    tokio::fs::write(&file_path, data).await?;
    
    // Kembalikan URL relatif
    Ok(format!("/uploads/{}", filename))
}

/// Fungsi untuk upload file dengan item_id, membuat folder berdasarkan ID item jika belum ada
pub async fn upload_file_with_item_id(
    mut payload: Multipart,
    config: web::Data<DriveConfig>,
    client: web::Data<Arc<Mutex<DriveClient>>>,
    item_id: uuid::Uuid,
) -> Result<String, Box<dyn std::error::Error>> {
    println!("[DEBUG] upload_file_with_item_id: Memulai proses upload untuk item {}", item_id);
    
    // Proses file upload
    while let Ok(Some(mut field)) = payload.try_next().await {
        // Dapatkan content disposition
        let content_disposition = field.content_disposition();
        
        // Dapatkan nama file
        let filename = content_disposition
            .get_filename()
            .map_or_else(|| Uuid::new_v4().to_string(), sanitize);
        
        // Buat nama file unik dengan timestamp dan UUID
        let uuid = Uuid::new_v4();
        let timestamp = chrono::Utc::now().timestamp();
        let file_ext = Path::new(&filename)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("jpg");
        
        let unique_filename = format!("{}-{}.{}", timestamp, uuid, file_ext);
        println!("[DEBUG] upload_file_with_item_id: Filename: {}", unique_filename);
        
        // Dapatkan content type
        let content_type_opt = field.content_type().map(|ct| ct.clone());
        println!("[DEBUG] upload_file_with_item_id: Content type: {:?}", content_type_opt);
        
        // Validasi tipe file
        if let Some(content_type) = &content_type_opt {
            if !config.allowed_types.contains(&content_type.to_string()) {
                println!("[WARN] upload_file_with_item_id: Tipe file tidak diizinkan: {}", content_type);
                return Err(format!("Tipe file tidak diizinkan: {}", content_type).into());
            }
        }
        
        // Baca data file
        let mut data = Vec::new();
        while let Some(chunk) = field.next().await {
            let chunk_data = chunk?;
            data.extend_from_slice(&chunk_data);
            
            // Cek ukuran file
            if data.len() > config.max_file_size {
                println!("[WARN] upload_file_with_item_id: Ukuran file melebihi batas: {} > {}", data.len(), config.max_file_size);
                return Err(format!("Ukuran file melebihi batas: {} > {}", data.len(), config.max_file_size).into());
            }
        }
        
        println!("[DEBUG] upload_file_with_item_id: File size: {} bytes", data.len());
        
        // Coba ambil client dari mutex
        let mut client_guard = client.lock().await;
        
        // Buat folder berdasarkan item_id jika belum ada
        let item_folder_name = format!("{}", item_id);
        println!("[DEBUG] upload_file_with_item_id: Mencari atau membuat folder {}", item_folder_name);
        
        // Cari folder berdasarkan nama item_id di root folder
        let token = match get_access_token(&mut client_guard).await {
            Ok(t) => t,
            Err(e) => {
                println!("[ERROR] Gagal mendapatkan access token: {}", e);
                return Err(format!("Gagal mendapatkan access token: {}", e).into());
            }
        };
        
        let query = format!("name = '{}' and mimeType = 'application/vnd.google-apps.folder' and '{}' in parents", 
                           item_folder_name, config.folder_id);
        println!("[DEBUG] Query pencarian folder: {}", query);
        
        // URL encode query parameter secara manual
        let encoded_query = query.replace(" ", "%20")
                               .replace("'", "%27")
                               .replace("=", "%3D")
                               .replace("(", "%28")
                               .replace(")", "%29");
        
        let search_url = format!("https://www.googleapis.com/drive/v3/files?q={}&fields=files(id,name)", 
                                encoded_query);
        println!("[DEBUG] URL pencarian folder: {}", search_url);
        
        let search_response = match client_guard.client.get(&search_url)
            .bearer_auth(&token)
            .send()
            .await {
                Ok(resp) => resp,
                Err(e) => {
                    println!("[ERROR] Gagal mengirim request pencarian folder: {}", e);
                    return Err(format!("Gagal mengirim request pencarian folder: {}", e).into());
                }
            };
        
        println!("[DEBUG] Status response pencarian folder: {}", search_response.status());
        
        let item_folder_id = if search_response.status().is_success() {
            let search_result = match search_response.json::<serde_json::Value>().await {
                Ok(json) => json,
                Err(e) => {
                    println!("[ERROR] Gagal parsing JSON response pencarian folder: {}", e);
                    return Err(format!("Gagal parsing JSON response pencarian folder: {}", e).into());
                }
            };
            
            println!("[DEBUG] Response JSON pencarian folder: {}", search_result);
            
            if let Some(files) = search_result["files"].as_array() {
                if !files.is_empty() {
                    // Folder sudah ada, gunakan ID yang ada
                    let folder_id = match files[0]["id"].as_str() {
                        Some(id) => id.to_string(),
                        None => {
                            println!("[ERROR] ID folder tidak ditemukan dalam response");
                            return Err("ID folder tidak ditemukan dalam response".into());
                        }
                    };
                    println!("[INFO] upload_file_with_item_id: Folder untuk item {} sudah ada dengan ID {}", item_id, folder_id);
                    folder_id
                } else {
                    // Folder belum ada, buat baru
                    println!("[INFO] upload_file_with_item_id: Folder untuk item {} belum ada, membuat baru", item_id);
                    
                    // Buat metadata folder
                    let folder_metadata = serde_json::json!({
                        "name": item_folder_name,
                        "mimeType": "application/vnd.google-apps.folder",
                        "parents": [config.folder_id]
                    });
                    
                    // Buat folder
                    println!("[DEBUG] Membuat folder baru dengan metadata: {}", folder_metadata);
                    let create_response = match client_guard.client.post("https://www.googleapis.com/drive/v3/files")
                        .query(&[("fields", "id")])
                        .bearer_auth(&token)
                        .json(&folder_metadata)
                        .send()
                        .await {
                            Ok(resp) => resp,
                            Err(e) => {
                                println!("[ERROR] Gagal mengirim request pembuatan folder: {}", e);
                                return Err(format!("Gagal mengirim request pembuatan folder: {}", e).into());
                            }
                        };
                    
                    println!("[DEBUG] Response status pembuatan folder: {}", create_response.status());
                    if !create_response.status().is_success() {
                        let error_text = match create_response.text().await {
                            Ok(text) => text,
                            Err(e) => format!("Tidak bisa membaca response error: {}", e)
                        };
                        println!("[ERROR] Gagal membuat folder: {}", error_text);
                        return Err(format!("Gagal membuat folder: {}", error_text).into());
                    }
                    
                    // Parse JSON response untuk mendapatkan folder ID
                    let folder_response = match create_response.json::<serde_json::Value>().await {
                        Ok(json) => json,
                        Err(e) => {
                            println!("[ERROR] Gagal parsing JSON response pembuatan folder: {}", e);
                            return Err(format!("Gagal parsing JSON response pembuatan folder: {}", e).into());
                        }
                    };
                    
                    println!("[DEBUG] Response JSON pembuatan folder: {}", folder_response);
                    
                    let new_folder_id = match folder_response["id"].as_str() {
                        Some(id) => id,
                        None => {
                            println!("[ERROR] Tidak bisa mendapatkan ID folder dari response: {}", folder_response);
                            return Err("Gagal mendapatkan ID folder dari response".into());
                        }
                    };
                    
                    println!("[INFO] Folder berhasil dibuat dengan ID: {}", new_folder_id);
                    
                    // Atur permission agar folder bisa diakses publik
                    let permission = serde_json::json!({
                        "role": "reader",
                        "type": "anyone"
                    });
                    
                    println!("[DEBUG] Mengatur permission publik untuk folder {}", new_folder_id);
                    let perm_response = match client_guard.client.post(&format!("https://www.googleapis.com/drive/v3/files/{}/permissions", new_folder_id))
                        .bearer_auth(&token)
                        .json(&permission)
                        .send()
                        .await {
                            Ok(resp) => resp,
                            Err(e) => {
                                println!("[ERROR] Gagal mengirim request permission: {}", e);
                                // Tetap lanjutkan meskipun permission gagal
                                println!("[WARN] Melanjutkan meskipun gagal mengatur permission");
                                return Ok(new_folder_id.to_string());
                            }
                        };
                    
                    println!("[DEBUG] Response status permission: {}", perm_response.status());
                    if !perm_response.status().is_success() {
                        let error_text = match perm_response.text().await {
                            Ok(text) => text,
                            Err(e) => format!("Tidak bisa membaca response error: {}", e)
                        };
                        println!("[WARN] Gagal mengatur permission: {}", error_text);
                        // Tetap lanjutkan meskipun permission gagal
                        println!("[WARN] Melanjutkan meskipun gagal mengatur permission");
                    } else {
                        println!("[INFO] Permission berhasil diatur untuk folder {}", new_folder_id);
                    }
                    
                    new_folder_id.to_string()
                }
            } else {
                return Err("Respons pencarian folder tidak valid".into());
            }
        } else {
            let error_text = match search_response.text().await {
                Ok(text) => text,
                Err(e) => format!("Tidak bisa membaca response error: {}", e)
            };
            println!("[ERROR] Gagal mencari folder: {}", error_text);
            return Err(format!("Gagal mencari folder: {}", error_text).into());
        };
        
        // Upload file ke folder item
        println!("[DEBUG] upload_file_with_item_id: Mengupload file ke folder item {}", item_folder_id);
        let file_id = match upload_to_drive(&mut client_guard, &item_folder_id, &unique_filename, data, content_type_opt).await {
            Ok(id) => id,
            Err(e) => {
                println!("[ERROR] Gagal mengupload file ke folder: {}", e);
                return Err(format!("Gagal mengupload file ke folder: {}", e).into());
            }
        };
        
        // Buat URL proxy lokal yang dapat diakses langsung oleh browser
        // Format: {base_url}/api/upload/proxy/drive/{file_id}
        let public_url = format!("{}/api/upload/proxy/drive/{}", config.base_url, file_id);
        println!("[DEBUG] URL publik (proxy): {}", public_url);
        
        return Ok(public_url);
    }
    
    Err("Tidak ada file yang diupload".into())
}

/// Handler untuk endpoint upload file
pub async fn upload_file_handler(
    payload: Multipart,
    config: web::Data<DriveConfig>,
    client: web::Data<Arc<Mutex<DriveClient>>>,
) -> Result<web::Json<serde_json::Value>, Error> {
    println!("[DEBUG] upload_file_handler: Memulai proses upload");
    
    // Coba upload ke Google Drive terlebih dahulu
    match upload_to_drive_or_local(payload, config, client).await {
        Ok(file_url) => {
            println!("[INFO] upload_file_handler: Upload berhasil, URL: {}", file_url);
            Ok(web::Json(serde_json::json!({
                "url": file_url
            })))
        },
        Err(e) => {
            println!("[ERROR] upload_file_handler: Error upload: {}", e);
            Err(actix_web::error::ErrorInternalServerError(format!(
                "Gagal upload file: {}", e
            )))
        },
    }
}

/// Upload file ke Google Drive dengan data file yang sudah diekstrak
/// Fungsi ini digunakan oleh update_item_with_image untuk mengupload file
/// yang sudah diekstrak dari payload multipart
pub async fn upload_file_with_item_id_field(
    filename: String,
    bytes: Vec<u8>,
    content_type: String,
    config: web::Data<DriveConfig>,
    client: web::Data<Arc<Mutex<DriveClient>>>,
    item_id: uuid::Uuid,
) -> Result<String, Box<dyn std::error::Error>> {
    println!("[DEBUG] Uploading file {} ({} bytes) for item {}", filename, bytes.len(), item_id);
    
    // Buat nama file unik dengan timestamp dan UUID
    let uuid = uuid::Uuid::new_v4();
    let timestamp = chrono::Utc::now().timestamp();
    let file_ext = Path::new(&filename)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("jpg");
    
    let unique_filename = format!("{}-{}.{}", timestamp, uuid, file_ext);
    println!("[DEBUG] Generated unique filename: {}", unique_filename);
    
    // Coba ambil client dari mutex
    let mut client_guard = client.lock().await;
    
    // Buat folder berdasarkan item_id jika belum ada
    let item_folder_name = format!("{}", item_id);
    println!("[DEBUG] Looking for or creating folder {}", item_folder_name);
    
    // Cari folder berdasarkan nama item_id di root folder
    let token = match get_access_token(&mut client_guard).await {
        Ok(t) => t,
        Err(e) => {
            println!("[ERROR] Failed to get access token: {}", e);
            return Err(format!("Failed to get access token: {}", e).into());
        }
    };
    
    let query = format!("name = '{}' and mimeType = 'application/vnd.google-apps.folder' and '{}' in parents", 
                       item_folder_name, config.folder_id);
    println!("[DEBUG] Folder search query: {}", query);
    
    // URL encode query parameter secara manual
    let encoded_query = query.replace(" ", "%20")
                           .replace("'", "%27")
                           .replace("=", "%3D")
                           .replace("(", "%28")
                           .replace(")", "%29");
    
    let search_url = format!("https://www.googleapis.com/drive/v3/files?q={}&fields=files(id,name)", 
                            encoded_query);
    
    let search_response = match client_guard.client.get(&search_url)
        .bearer_auth(&token)
        .send()
        .await {
            Ok(resp) => resp,
            Err(e) => {
                println!("[ERROR] Failed to send folder search request: {}", e);
                return Err(format!("Failed to send folder search request: {}", e).into());
            }
        };
    
    let item_folder_id = if search_response.status().is_success() {
        let search_result = match search_response.json::<serde_json::Value>().await {
            Ok(json) => json,
            Err(e) => {
                println!("[ERROR] Failed to parse folder search JSON response: {}", e);
                return Err(format!("Failed to parse folder search JSON response: {}", e).into());
            }
        };
        
        if let Some(files) = search_result["files"].as_array() {
            if !files.is_empty() {
                // Folder sudah ada, gunakan ID yang ada
                let folder_id = match files[0]["id"].as_str() {
                    Some(id) => id.to_string(),
                    None => {
                        println!("[ERROR] Folder ID not found in response");
                        return Err("Folder ID not found in response".into());
                    }
                };
                println!("[INFO] Folder for item {} already exists with ID {}", item_id, folder_id);
                folder_id
            } else {
                // Folder belum ada, buat baru
                println!("[INFO] Folder for item {} does not exist, creating new one", item_id);
                
                // Buat metadata folder
                let folder_metadata = serde_json::json!({
                    "name": item_folder_name,
                    "mimeType": "application/vnd.google-apps.folder",
                    "parents": [config.folder_id]
                });
                
                // Buat folder
                let create_response = match client_guard.client.post("https://www.googleapis.com/drive/v3/files")
                    .query(&[("fields", "id")])
                    .bearer_auth(&token)
                    .json(&folder_metadata)
                    .send()
                    .await {
                        Ok(resp) => resp,
                        Err(e) => {
                            println!("[ERROR] Failed to send folder creation request: {}", e);
                            return Err(format!("Failed to send folder creation request: {}", e).into());
                        }
                    };
                
                if !create_response.status().is_success() {
                    let error_text = match create_response.text().await {
                        Ok(text) => text,
                        Err(e) => format!("Could not read error response: {}", e)
                    };
                    println!("[ERROR] Failed to create folder: {}", error_text);
                    return Err(format!("Failed to create folder: {}", error_text).into());
                }
                
                // Parse JSON response untuk mendapatkan folder ID
                let folder_response = match create_response.json::<serde_json::Value>().await {
                    Ok(json) => json,
                    Err(e) => {
                        println!("[ERROR] Failed to parse folder creation JSON response: {}", e);
                        return Err(format!("Failed to parse folder creation JSON response: {}", e).into());
                    }
                };
                
                let new_folder_id = match folder_response["id"].as_str() {
                    Some(id) => id,
                    None => {
                        println!("[ERROR] Could not get folder ID from response: {}", folder_response);
                        return Err("Failed to get folder ID from response".into());
                    }
                };
                
                println!("[INFO] Folder successfully created with ID: {}", new_folder_id);
                
                // Atur permission agar folder bisa diakses publik
                let permission = serde_json::json!({
                    "role": "reader",
                    "type": "anyone"
                });
                
                println!("[DEBUG] Setting public permission for folder {}", new_folder_id);
                let perm_response = match client_guard.client.post(&format!("https://www.googleapis.com/drive/v3/files/{}/permissions", new_folder_id))
                    .bearer_auth(&token)
                    .json(&permission)
                    .send()
                    .await {
                        Ok(resp) => resp,
                        Err(e) => {
                            println!("[ERROR] Failed to send permission request: {}", e);
                            // Continue even if permission setting fails
                            println!("[WARN] Continuing despite permission setting failure");
                            return Ok(new_folder_id.to_string());
                        }
                    };
                
                if !perm_response.status().is_success() {
                    let error_text = match perm_response.text().await {
                        Ok(text) => text,
                        Err(e) => format!("Could not read error response: {}", e)
                    };
                    println!("[WARN] Failed to set permission: {}", error_text);
                    // Continue even if permission setting fails
                    println!("[WARN] Continuing despite permission setting failure");
                } else {
                    println!("[INFO] Permission successfully set for folder {}", new_folder_id);
                }
                
                new_folder_id.to_string()
            }
        } else {
            return Err("Invalid folder search response".into());
        }
    } else {
        let error_text = match search_response.text().await {
            Ok(text) => text,
            Err(e) => format!("Could not read error response: {}", e)
        };
        println!("[ERROR] Failed to search for folder: {}", error_text);
        return Err(format!("Failed to search for folder: {}", error_text).into());
    };
    
    // Upload file ke folder item
    println!("[DEBUG] Uploading file to item folder {}", item_folder_id);
    
    // Prepare file metadata
    let metadata = serde_json::json!({
        "name": unique_filename,
        "parents": [item_folder_id]
    });
    
    // Determine a more specific MIME type based on file extension
    let mime_type = if content_type == "application/octet-stream" {
        // Try to determine MIME type from file extension
        match file_ext.to_lowercase().as_str() {
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "gif" => "image/gif",
            "pdf" => "application/pdf",
            "doc" | "docx" => "application/msword",
            "xls" | "xlsx" => "application/vnd.ms-excel",
            "txt" => "text/plain",
            _ => "image/jpeg"  // Default to image/jpeg if unknown
        }
    } else {
        &content_type
    };
    
    println!("[DEBUG] Using MIME type: {}", mime_type);
    
    // Gunakan pendekatan alternatif untuk upload file
    // Buat metadata file dengan JSON
    let metadata_json = serde_json::json!({
        "name": unique_filename,
        "parents": [item_folder_id]
    });
    
    // Buat URL untuk upload file
    let upload_url = format!("https://www.googleapis.com/upload/drive/v3/files?uploadType=multipart&fields=id");
    
    // Buat multipart body secara manual
    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    
    // Buat body untuk multipart request
    let mut body = Vec::new();
    
    // Tambahkan metadata part
    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice("Content-Type: application/json; charset=UTF-8\r\n\r\n".as_bytes());
    body.extend_from_slice(serde_json::to_string(&metadata_json).unwrap().as_bytes());
    body.extend_from_slice("\r\n".as_bytes());
    
    // Tambahkan file part
    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(format!("Content-Type: {}\r\n\r\n", mime_type).as_bytes());
    body.extend_from_slice(&bytes);
    body.extend_from_slice("\r\n".as_bytes());
    
    // Tutup boundary
    body.extend_from_slice(format!("--{}--", boundary).as_bytes());
    
    println!("[DEBUG] Uploading file with manual multipart request");
    
    // Buat request dengan body yang sudah dibuat
    let upload_response = match client_guard.client.post(&upload_url)
        .header("Content-Type", format!("multipart/related; boundary={}", boundary))
        .bearer_auth(&token)
        .body(body)
        .send()
        .await {
            Ok(resp) => resp,
            Err(e) => {
                println!("[ERROR] Failed to upload file: {}", e);
                return Err(format!("Failed to upload file: {}", e).into());
            }
        };
    
    if !upload_response.status().is_success() {
        let error_text = match upload_response.text().await {
            Ok(text) => text,
            Err(e) => format!("Could not read error response: {}", e)
        };
        println!("[ERROR] Failed to upload file: {}", error_text);
        return Err(format!("Failed to upload file: {}", error_text).into());
    }
    
    // Parse response to get file ID
    let upload_result = match upload_response.json::<serde_json::Value>().await {
        Ok(json) => json,
        Err(e) => {
            println!("[ERROR] Failed to parse upload JSON response: {}", e);
            return Err(format!("Failed to parse upload JSON response: {}", e).into());
        }
    };
    
    let file_id = match upload_result["id"].as_str() {
        Some(id) => id,
        None => {
            println!("[ERROR] Could not get file ID from response: {}", upload_result);
            return Err("Failed to get file ID from response".into());
        }
    };
    
    println!("[INFO] File successfully uploaded with ID: {}", file_id);
    
    // Atur permission file agar dapat diakses publik
    println!("[DEBUG] Setting public permission for file {}", file_id);
    let permission = serde_json::json!({
        "role": "reader",
        "type": "anyone"
    });
    
    // Atur permission file
    let perm_response = client_guard.client.post(&format!("https://www.googleapis.com/drive/v3/files/{}/permissions", file_id))
        .bearer_auth(&token)
        .json(&permission)
        .send()
        .await;
        
    // Handle response
    match perm_response {
        Ok(resp) => {
            if !resp.status().is_success() {
                let error_text = match resp.text().await {
                    Ok(text) => text,
                    Err(e) => format!("Could not read error response: {}", e)
                };
                println!("[WARN] Failed to set file permission: {}", error_text);
            } else {
                println!("[INFO] File permission set successfully");
            }
        },
        Err(e) => {
            println!("[ERROR] Failed to set file permission: {}", e);
            println!("[WARN] Continuing despite permission setting failure");
            // Continue anyway, but URL might not be accessible
        }
    };
    
    // Kembalikan hanya file ID saja, bukan URL lengkap
    // Frontend akan memformat URL sesuai kebutuhan melalui fungsi formatPhotoUrl
    let public_url = file_id.to_string();
    println!("[DEBUG] File ID (untuk frontend): {}", public_url);
    
    Ok(public_url)
}
