// Import beberapa tipe dan trait dari actix_web yang dibutuhkan untuk membuat extractor custom
use actix_web::{dev::Payload, Error, FromRequest, HttpRequest};
// Import future Ready untuk mengembalikan hasil secara langsung (synchronous)
use futures::future::{ready, Ready};
// Derive trait Debug, Deserialize (untuk parsing dari JWT), dan Clone untuk struct Claims
#[derive(Debug, serde::Deserialize, Clone)]
// Struct Claims adalah representasi data yang ada di dalam JWT (JSON Web Token)
pub struct Claims {
    pub sub: String, // "sub" biasanya adalah user id atau identifier unik lain
    #[allow(dead_code)] // Attribute ini agar Rust tidak warning kalau exp tidak dipakai
    pub exp: usize,    // "exp" adalah waktu kadaluarsa token (epoch timestamp)
    pub role: String,  // "role" biasanya untuk otorisasi (misal: admin, user, dll)
}

// Import crate jsonwebtoken untuk proses decode JWT
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};

// Implementasi trait FromRequest agar Claims bisa langsung diekstrak dari request secara otomatis
impl FromRequest for Claims {
    // DEBUG: log setiap kali extractor ini dipanggil
    // (log ada di dalam from_request di bawah)

    // Tipe error yang akan dikembalikan jika gagal
    type Error = Error;
    // Tipe future yang dikembalikan (langsung jadi, tidak async/await)
    type Future = Ready<Result<Self, Self::Error>>;

    // Fungsi utama extractor: mengambil data JWT dari request
    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        // Print ke konsol path yang sedang diakses (untuk debug)
        println!("[JWT Extractor] called for path: {}", req.path());
        // Ambil token dari header Authorization (Bearer <token>) atau dari cookie bernama 'token'
        let token_opt = req
            .headers() // ambil semua header
            .get("Authorization") // cari header Authorization
            .and_then(|h| h.to_str().ok()) // konversi ke string jika ada
            .and_then(|auth| {
                // Jika header mulai dengan "Bearer ", ambil tokennya saja
                if auth.starts_with("Bearer ") {
                    Some(auth[7..].to_string()) // ambil substring setelah "Bearer "
                } else {
                    None // jika tidak, return None
                }
            })
            .or_else(|| {
                // Jika tidak ada di header, coba ambil dari cookie bernama "token"
                req.cookie("token").map(|c| c.value().to_string())
            });

        // Jika token ditemukan
        if let Some(token) = token_opt {
            // Print token ke konsol untuk debug
            println!("[JWT Extractor] Token found: {}", &token);
            // Ambil secret dari environment variable JWT_SECRET (harus di-set di .env atau environment)
            let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET harus di-set");
            // Buat decoding key dari secret
            let decoding_key = DecodingKey::from_secret(secret.as_bytes());
            // Set algoritma validasi JWT ke HS256
            let validation = Validation::new(Algorithm::HS256);
            // Decode token menjadi struct Claims
            match decode::<Claims>(&token, &decoding_key, &validation) {
                Ok(data) => {
                    // Jika sukses, print data ke konsol
                    println!("[JWT Extractor] JWT decode success: sub={}, role={}", data.claims.sub, data.claims.role);
                    // Return data.claims sebagai hasil extractor
                    ready(Ok(data.claims))
                },
                Err(e) => {
                    // Jika gagal decode (token invalid/expired), print error ke konsol
                    println!("[JWT Extractor] JWT decode error: {:?}", e);
                    // Return error unauthorized (401) dengan pesan JSON
                    ready(Err(actix_web::error::ErrorUnauthorized(serde_json::json!({"message": "Invalid JWT"}))))
                },
            }
        } else {
            // Jika token tidak ditemukan di header maupun cookie
            println!("[JWT Extractor] No JWT token found in header or cookie");
            // Return error unauthorized (401) dengan pesan JSON
            ready(Err(actix_web::error::ErrorUnauthorized(serde_json::json!({"message": "No JWT token"}))))
        }
    }
}
