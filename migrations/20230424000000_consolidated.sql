-- Consolidated migration file for Actisol
-- This file combines all migrations into a single file

-- Dynamic tables for previously-enum values
CREATE TABLE IF NOT EXISTS categories (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name VARCHAR(64) NOT NULL UNIQUE,
  description TEXT
);

CREATE TABLE IF NOT EXISTS item_sources (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name VARCHAR(64) NOT NULL UNIQUE,
  description TEXT
);

CREATE TABLE IF NOT EXISTS conditions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name VARCHAR(32) NOT NULL UNIQUE,
  description TEXT
);

CREATE TABLE IF NOT EXISTS procurement_statuses (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name VARCHAR(32) NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS user_roles (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name VARCHAR(32) NOT NULL UNIQUE,
  description TEXT
);

-- Insert initial values for lookup tables
INSERT INTO categories (name) 
VALUES ('electronics'), ('prayer'), ('furniture')
ON CONFLICT (name) DO NOTHING;

INSERT INTO item_sources (name) 
VALUES ('existing'), ('donation'), ('procurement')
ON CONFLICT (name) DO NOTHING;

INSERT INTO conditions (name) 
VALUES ('good'), ('damaged'), ('lost')
ON CONFLICT (name) DO NOTHING;

INSERT INTO procurement_statuses (name) 
VALUES ('pending'), ('approved'), ('rejected'), ('purchased')
ON CONFLICT (name) DO NOTHING;

INSERT INTO user_roles (name, description) 
VALUES 
  ('admin', 'Akses penuh sistem'),
  ('staff', 'Manajemen inventaris dasar'),
  ('manager', 'Manajer departemen dengan hak persetujuan'),
  ('caretaker', 'Bertanggung jawab untuk pemeliharaan dan pelaporan kondisi'),
  ('borrower', 'Hanya dapat melihat dan meminta untuk meminjam barang'),
  ('procurement_officer', 'Dapat mengelola permintaan pengadaan')
ON CONFLICT (name) DO UPDATE SET description = EXCLUDED.description;

-- Table: locations
CREATE TABLE IF NOT EXISTS locations (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name VARCHAR(64) NOT NULL UNIQUE,
  description TEXT
);

-- Table: users
CREATE TABLE IF NOT EXISTS users (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  password_hash TEXT,
  email VARCHAR(128) UNIQUE,
  name VARCHAR(64) NOT NULL UNIQUE,
  phone_number VARCHAR(16),
  avatar_url TEXT,
  role_id UUID NOT NULL REFERENCES user_roles(id),
  created_at TIMESTAMPTZ DEFAULT now()
);

-- Table: donations
CREATE TABLE IF NOT EXISTS donations (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  donor_name VARCHAR(64) NOT NULL,
  donor_email VARCHAR(128),
  item_name VARCHAR(128) NOT NULL,
  category_id UUID NOT NULL REFERENCES categories(id),
  quantity INTEGER NOT NULL DEFAULT 1,
  condition_id UUID NOT NULL REFERENCES conditions(id),
  photo_url TEXT,
  status VARCHAR(16) NOT NULL DEFAULT 'pending',
  admin_note TEXT,
  created_at TIMESTAMPTZ DEFAULT now()
);

-- Table: procurements
CREATE TABLE IF NOT EXISTS procurements (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  item_name VARCHAR(128) NOT NULL,
  category_id UUID NOT NULL REFERENCES categories(id),
  quantity INTEGER NOT NULL DEFAULT 1,
  reason TEXT,
  status_id UUID NOT NULL REFERENCES procurement_statuses(id),
  requested_by UUID REFERENCES users(id),
  approved_by UUID REFERENCES users(id),
  admin_note TEXT,
  created_at TIMESTAMPTZ DEFAULT now(),
  purchased_at TIMESTAMPTZ
);

-- Item status table
CREATE TABLE IF NOT EXISTS item_statuses (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name VARCHAR(64) NOT NULL UNIQUE,
  description TEXT,
  color VARCHAR(32) -- For UI color coding
);

-- Insert initial item statuses
INSERT INTO item_statuses (name, description, color) VALUES 
('active', 'Item tersedia untuk digunakan', 'green'),
('maintenance', 'Item sedang dalam pemeliharaan', 'orange'),
('borrowed', 'Item sedang dipinjam', 'blue'),
('reserved', 'Item direservasi untuk penggunaan mendatang', 'purple'),
('damaged', 'Item rusak tapi masih bisa digunakan', 'yellow'),
('unusable', 'Item rusak tidak dapat digunakan', 'red')
ON CONFLICT (name) DO UPDATE SET 
  description = EXCLUDED.description,
  color = EXCLUDED.color;

-- Table: items
CREATE TABLE IF NOT EXISTS items (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name VARCHAR(128) NOT NULL,
  category_id UUID NOT NULL REFERENCES categories(id),
  quantity INTEGER NOT NULL DEFAULT 1,
  condition_id UUID NOT NULL REFERENCES conditions(id),
  location_id UUID REFERENCES locations(id) ON DELETE SET NULL,
  photo_url TEXT,
  source_id UUID NOT NULL REFERENCES item_sources(id),
  donor_id UUID REFERENCES donations(id),
  procurement_id UUID REFERENCES procurements(id),
  created_at TIMESTAMPTZ DEFAULT now()
);

-- Add status_id to items table if it doesn't exist
DO $$ 
BEGIN
  IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'items' AND column_name = 'status_id') THEN
    ALTER TABLE items ADD COLUMN status_id UUID REFERENCES item_statuses(id);
    
    -- Set default status for existing items
    UPDATE items SET status_id = (SELECT id FROM item_statuses WHERE name = 'active')
    WHERE status_id IS NULL;
    
    -- Make status_id NOT NULL after updating existing records
    ALTER TABLE items ALTER COLUMN status_id SET NOT NULL;
  END IF;
END $$;

-- Table: item_logs
CREATE TABLE IF NOT EXISTS item_logs (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  item_id UUID REFERENCES items(id) ON DELETE CASCADE,
  action VARCHAR(32) NOT NULL,
  before JSONB,
  after JSONB,
  note TEXT,
  by UUID REFERENCES users(id),
  created_at TIMESTAMPTZ DEFAULT now()
);

-- Table: movement_history
CREATE TABLE IF NOT EXISTS movement_history (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  item_id UUID REFERENCES items(id) ON DELETE CASCADE,
  from_location_id UUID REFERENCES locations(id),
  to_location_id UUID REFERENCES locations(id),
  moved_by UUID REFERENCES users(id),
  reason TEXT,
  moved_at TIMESTAMPTZ DEFAULT now()
);

-- Create permissions table
CREATE TABLE IF NOT EXISTS permissions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name VARCHAR(64) NOT NULL UNIQUE,
  description TEXT
);

-- Insert basic permissions
INSERT INTO permissions (name, description) VALUES
('view_items', 'Dapat melihat item inventaris'),
('add_items', 'Dapat menambahkan item inventaris baru'),
('edit_items', 'Dapat mengedit item inventaris yang ada'),
('delete_items', 'Dapat menghapus item inventaris'),
('manage_categories', 'Dapat mengelola kategori item'),
('manage_locations', 'Dapat mengelola lokasi'),
('manage_users', 'Dapat mengelola akun pengguna'),
('approve_procurements', 'Dapat menyetujui permintaan pengadaan'),
('manage_donations', 'Dapat mengelola donasi'),
('view_reports', 'Dapat melihat laporan sistem'),
('manage_item_status', 'Dapat mengubah status item'),
('borrow_items', 'Dapat meminjam item'),
('admin_access', 'Akses administratif penuh'),
('manage_permissions', 'Dapat mengelola izin peran'),
('manage_roles', 'Dapat mengelola peran pengguna'),
('approve_borrowings', 'Dapat menyetujui permintaan peminjaman item'),
('manage_borrowings', 'Dapat mengelola semua peminjaman'),
('view_all_borrowings', 'Dapat melihat semua peminjaman')
ON CONFLICT (name) DO UPDATE SET description = EXCLUDED.description;

-- Create role-permission mapping table
CREATE TABLE IF NOT EXISTS role_permissions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  role_id UUID REFERENCES user_roles(id) ON DELETE CASCADE,
  permission_id UUID REFERENCES permissions(id) ON DELETE CASCADE,
  UNIQUE(role_id, permission_id)
);

-- Assign permissions to roles
-- Admin role (all permissions)
-- First, clear existing permissions for the admin role to avoid duplicates
DELETE FROM role_permissions
WHERE role_id = (SELECT id FROM user_roles WHERE name = 'admin');

-- Then assign all permissions to admin
INSERT INTO role_permissions (role_id, permission_id)
SELECT 
  (SELECT id FROM user_roles WHERE name = 'admin'),
  id
FROM permissions;

-- Staff role
-- Clear existing permissions for this role to avoid duplicates
DELETE FROM role_permissions
WHERE role_id = (SELECT id FROM user_roles WHERE name = 'staff');

-- Assign specific permissions
INSERT INTO role_permissions (role_id, permission_id)
SELECT 
  (SELECT id FROM user_roles WHERE name = 'staff'),
  id
FROM permissions
WHERE name IN ('view_items', 'add_items', 'edit_items', 'view_reports', 'manage_item_status', 'borrow_items');

-- Manager role
-- Clear existing permissions for this role to avoid duplicates
DELETE FROM role_permissions
WHERE role_id = (SELECT id FROM user_roles WHERE name = 'manager');

-- Assign specific permissions
INSERT INTO role_permissions (role_id, permission_id)
SELECT 
  (SELECT id FROM user_roles WHERE name = 'manager'),
  id
FROM permissions
WHERE name IN ('view_items', 'add_items', 'edit_items', 'delete_items', 'manage_categories', 
             'manage_locations', 'approve_procurements', 'view_reports', 'manage_item_status',
             'approve_borrowings', 'view_all_borrowings');

-- Caretaker role
-- Clear existing permissions for this role to avoid duplicates
DELETE FROM role_permissions
WHERE role_id = (SELECT id FROM user_roles WHERE name = 'caretaker');

-- Assign specific permissions
INSERT INTO role_permissions (role_id, permission_id)
SELECT 
  (SELECT id FROM user_roles WHERE name = 'caretaker'),
  id
FROM permissions
WHERE name IN ('view_items', 'edit_items', 'manage_item_status');

-- Borrower role
-- Clear existing permissions for this role to avoid duplicates
DELETE FROM role_permissions
WHERE role_id = (SELECT id FROM user_roles WHERE name = 'borrower');

-- Assign specific permissions
INSERT INTO role_permissions (role_id, permission_id)
SELECT 
  (SELECT id FROM user_roles WHERE name = 'borrower'),
  id
FROM permissions
WHERE name IN ('view_items', 'borrow_items');

-- Procurement officer role
-- Clear existing permissions for this role to avoid duplicates
DELETE FROM role_permissions
WHERE role_id = (SELECT id FROM user_roles WHERE name = 'procurement_officer');

-- Assign specific permissions
INSERT INTO role_permissions (role_id, permission_id)
SELECT 
  (SELECT id FROM user_roles WHERE name = 'procurement_officer'),
  id
FROM permissions
WHERE name IN ('view_items', 'add_items', 'manage_categories', 'approve_procurements', 'view_reports');

-- Create item borrowing table to track borrowed items
CREATE TABLE IF NOT EXISTS item_borrowings (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  item_id UUID NOT NULL REFERENCES items(id) ON DELETE CASCADE,
  borrower_id UUID NOT NULL REFERENCES users(id),
  quantity INTEGER NOT NULL DEFAULT 1,
  borrowed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  expected_return_date TIMESTAMPTZ NOT NULL,
  actual_return_date TIMESTAMPTZ,
  approved_by UUID REFERENCES users(id),
  notes TEXT,
  status VARCHAR(32) NOT NULL DEFAULT 'pending' -- pending, approved, rejected, returned, overdue
);

-- Create indexes for better performance
DO $$ 
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_item_borrowings_item_id') THEN
    CREATE INDEX idx_item_borrowings_item_id ON item_borrowings(item_id);
  END IF;
  
  IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_item_borrowings_borrower_id') THEN
    CREATE INDEX idx_item_borrowings_borrower_id ON item_borrowings(borrower_id);
  END IF;
  
  IF NOT EXISTS (SELECT 1 FROM pg_indexes WHERE indexname = 'idx_items_status_id') AND 
     EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'items' AND column_name = 'status_id') THEN
    CREATE INDEX idx_items_status_id ON items(status_id);
  END IF;
END $$;
