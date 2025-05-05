-- Migration: Tambah tabel donors dan procurements untuk kebutuhan export inventory

CREATE TABLE IF NOT EXISTS donors (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL
    -- Tambah kolom lain jika diperlukan, misal kontak, alamat, dsb
);

CREATE TABLE IF NOT EXISTS procurements (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL
    -- Tambah kolom lain jika diperlukan, misal tanggal, deskripsi, dsb
);
