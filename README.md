# Actisol Backend API

Backend API untuk aplikasi Actisol, dibangun dengan Rust dan Actix-Web.

## Fitur

- Autentikasi dengan JWT
- CRUD operasi untuk items, users, dan data lainnya
- Upload gambar dengan Google Drive Storage
- API RESTful
- Database PostgreSQL

## Persyaratan

- Rust 1.70.0 atau lebih baru
- PostgreSQL
- Akun Google Cloud dengan Google Drive API yang diaktifkan

## Instalasi

1. Clone repositori ini
2. Setup database PostgreSQL
3. Salin file `.env.example` ke `.env` dan sesuaikan konfigurasi
4. Jalankan dengan perintah:

```bash
cargo run
```

## Konfigurasi

Aplikasi ini menggunakan file `.env` untuk konfigurasi. Berikut adalah variabel yang perlu dikonfigurasi:

```
FRONTEND_URL=https://example.com
JWT_SECRET=your_jwt_secret

DATABASE_URL=postgresql://user:password@localhost:5432/dbname

# Google Drive Configuration
GOOGLE_CREDENTIALS_JSON={"type":"service_account",...}
GOOGLE_DRIVE_FOLDER_ID=your_folder_id
GOOGLE_DRIVE_PUBLIC_URL=https://drive.google.com/uc?export=view&id=
```

## Setup Google Drive untuk Penyimpanan Gambar

Aplikasi ini menggunakan Google Drive sebagai solusi penyimpanan gambar. Berikut adalah langkah-langkah untuk mengatur Google Drive API:

### 1. Membuat Project di Google Cloud

1. Kunjungi [Google Cloud Console](https://console.cloud.google.com/)
2. Buat project baru (atau gunakan yang sudah ada)
3. Catat Project ID yang akan digunakan nanti

### 2. Mengaktifkan Google Drive API

1. Di sidebar kiri, pilih "APIs & Services" > "Library"
2. Cari "Google Drive API"
3. Klik "Enable"

### 3. Membuat Service Account

1. Di sidebar kiri, pilih "IAM & Admin" > "Service Accounts"
2. Klik "Create Service Account"
3. Isi nama service account dan deskripsi
4. Berikan peran "Editor" pada service account
5. Klik "Done"

### 4. Membuat Key untuk Service Account

1. Pada daftar service account, klik service account yang baru dibuat
2. Pilih tab "Keys"
3. Klik "Add Key" > "Create new key"
4. Pilih format "JSON"
5. Klik "Create" untuk mengunduh file key

### 5. Menyiapkan Folder Google Drive

1. Buka [Google Drive](https://drive.google.com/)
2. Buat folder baru untuk menyimpan gambar
3. Klik kanan pada folder dan pilih "Share"
4. Bagikan folder dengan email service account yang baru dibuat
5. Berikan akses "Editor" pada service account

### 6. Mendapatkan Folder ID

1. Buka folder yang telah dibuat
2. Perhatikan URL di browser:
   `https://drive.google.com/drive/folders/FOLDER_ID`
3. Salin FOLDER_ID dari URL

### 7. Konfigurasi di File .env

1. Buka file JSON key yang diunduh
2. Salin seluruh isi file sebagai nilai untuk `GOOGLE_CREDENTIALS_JSON`
3. Salin Folder ID sebagai nilai untuk `GOOGLE_DRIVE_FOLDER_ID`
4. Format URL publik yang digunakan adalah `https://drive.google.com/file/d/{FILE_ID}/view`

Contoh:

```
GOOGLE_CREDENTIALS_JSON={"type":"service_account","project_id":"your-project-id","private_key_id":"your-key-id","private_key":"-----BEGIN PRIVATE KEY-----\nYOUR_PRIVATE_KEY\n-----END PRIVATE KEY-----\n","client_email":"your-service-account@your-project-id.iam.gserviceaccount.com","client_id":"your-client-id","auth_uri":"https://accounts.google.com/o/oauth2/auth","token_uri":"https://oauth2.googleapis.com/token","auth_provider_x509_cert_url":"https://www.googleapis.com/oauth2/v1/certs","client_x509_cert_url":"https://www.googleapis.com/robot/v1/metadata/x509/your-service-account%40your-project-id.iam.gserviceaccount.com","universe_domain":"googleapis.com"}
GOOGLE_DRIVE_FOLDER_ID=1AbCdEfGhIjKlMnOpQrStUvWxYz
GOOGLE_DRIVE_PUBLIC_URL=https://drive.google.com/uc?export=view&id=
```

### Catatan Penting

- Jangan pernah menyimpan file credentials JSON di repositori publik
- Google Drive menyediakan 15GB penyimpanan gratis
- Jika `GOOGLE_DRIVE_FOLDER_ID` tidak diisi, aplikasi akan otomatis membuat folder baru
- Gambar yang diupload akan diatur sebagai publik agar dapat diakses melalui URL

## Penggunaan API

### Upload Gambar

```
POST /api/upload
Content-Type: multipart/form-data

file: [binary data]
```

Respons:

```json
{
  "url": "https://drive.google.com/file/d/FILE_ID/view"
}
```

## Lisensi

MIT
