-- Dynamic tables for previously-enum values
CREATE TABLE categories (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name VARCHAR(64) NOT NULL UNIQUE
);
CREATE TABLE item_sources (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name VARCHAR(64) NOT NULL UNIQUE
);
CREATE TABLE conditions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name VARCHAR(32) NOT NULL UNIQUE
);
CREATE TABLE procurement_statuses (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name VARCHAR(32) NOT NULL UNIQUE
);
CREATE TABLE user_roles (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name VARCHAR(32) NOT NULL UNIQUE
);

-- Optionally, insert initial values for each table (uncomment if needed)
INSERT INTO categories (name) VALUES ('electronics'), ('prayer'), ('furniture');
INSERT INTO item_sources (name) VALUES ('existing'), ('donation'), ('procurement');
INSERT INTO conditions (name) VALUES ('good'), ('damaged'), ('lost');
INSERT INTO procurement_statuses (name) VALUES ('pending'), ('approved'), ('rejected'), ('purchased');
INSERT INTO user_roles (name) VALUES ('admin'), ('staff');


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
  role_id UUID NOT NULL REFERENCES user_roles(id),
  created_at TIMESTAMPTZ DEFAULT now()
);

-- Table: donations
CREATE TABLE donations (
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
CREATE TABLE procurements (
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

-- Table: items
CREATE TABLE items (
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