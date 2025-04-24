-- Enums
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'category') THEN
        CREATE TYPE category AS ENUM ('electronics', 'prayer', 'furniture');
    END IF;
END$$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'item_source') THEN
        CREATE TYPE item_source AS ENUM ('existing', 'donation', 'procurement');
    END IF;
END$$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'condition') THEN
        CREATE TYPE condition AS ENUM ('good', 'damaged', 'lost');
    END IF;
END$$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'procurement_status') THEN
        CREATE TYPE procurement_status AS ENUM ('pending', 'approved', 'rejected', 'purchased');
    END IF;
END$$;
DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'user_role') THEN
        CREATE TYPE user_role AS ENUM ('admin', 'staff');
    END IF;
END$$;

-- Table: locations
CREATE TABLE locations (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name VARCHAR(64) NOT NULL UNIQUE,
  description TEXT
);

-- Table: users
CREATE TABLE users (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  password_hash TEXT,
  email VARCHAR(128) UNIQUE,
  name VARCHAR(64) NOT NULL UNIQUE,
  phone_number VARCHAR(16),
  avatar_url TEXT,
  role user_role NOT NULL DEFAULT 'staff',
  created_at TIMESTAMPTZ DEFAULT now()
);

-- Table: donations
CREATE TABLE donations (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  donor_name VARCHAR(64) NOT NULL,
  donor_email VARCHAR(128),
  item_name VARCHAR(128) NOT NULL,
  category category NOT NULL,
  quantity INTEGER NOT NULL DEFAULT 1,
  condition condition NOT NULL DEFAULT 'good',
  photo_url TEXT,
  status VARCHAR(16) NOT NULL DEFAULT 'pending',
  admin_note TEXT,
  created_at TIMESTAMPTZ DEFAULT now()
);

-- Table: procurements
CREATE TABLE procurements (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  item_name VARCHAR(128) NOT NULL,
  category category NOT NULL,
  quantity INTEGER NOT NULL DEFAULT 1,
  reason TEXT,
  status procurement_status NOT NULL DEFAULT 'pending',
  requested_by UUID REFERENCES users(id),
  approved_by UUID REFERENCES users(id),
  admin_note TEXT,
  created_at TIMESTAMPTZ DEFAULT now(),
  purchased_at TIMESTAMPTZ
);

-- Table: items
CREATE TABLE items (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name VARCHAR(128) NOT NULL,
  category category NOT NULL,
  quantity INTEGER NOT NULL DEFAULT 1,
  condition condition NOT NULL DEFAULT 'good',
  location_id UUID REFERENCES locations(id) ON DELETE SET NULL,
  photo_url TEXT,
  source item_source NOT NULL,
  donor_id UUID REFERENCES donations(id),
  procurement_id UUID REFERENCES procurements(id),
  created_at TIMESTAMPTZ DEFAULT now()
);

-- Table: item_logs
CREATE TABLE item_logs (
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
CREATE TABLE movement_history (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  item_id UUID REFERENCES items(id) ON DELETE CASCADE,
  from_location_id UUID REFERENCES locations(id),
  to_location_id UUID REFERENCES locations(id),
  moved_by UUID REFERENCES users(id),
  reason TEXT,
  moved_at TIMESTAMPTZ DEFAULT now()
);