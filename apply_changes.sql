-- Add item status table
CREATE TABLE IF NOT EXISTS item_statuses (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name VARCHAR(64) NOT NULL UNIQUE,
  description TEXT,
  color VARCHAR(32) -- For UI color coding
);

-- Insert initial item statuses
INSERT INTO item_statuses (name, description, color) VALUES 
('active', 'Item is available for use', 'green'),
('maintenance', 'Item is under maintenance', 'orange'),
('borrowed', 'Item is currently borrowed', 'blue'),
('reserved', 'Item is reserved for future use', 'purple'),
('damaged', 'Item is damaged but still usable', 'yellow'),
('unusable', 'Item is damaged beyond use', 'red')
ON CONFLICT (name) DO NOTHING;

-- Add status_id to items table if it doesn't exist
DO $$ 
BEGIN
  IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'items' AND column_name = 'status_id') THEN
    ALTER TABLE items ADD COLUMN status_id UUID REFERENCES item_statuses(id);
    
    -- Set default status for existing items
    UPDATE items SET status_id = (SELECT id FROM item_statuses WHERE name = 'active');
    
    -- Make status_id NOT NULL after updating existing records
    ALTER TABLE items ALTER COLUMN status_id SET NOT NULL;
  END IF;
END $$;

-- Add description column to user_roles if it doesn't exist
DO $$ 
BEGIN
  IF NOT EXISTS (SELECT 1 FROM information_schema.columns WHERE table_name = 'user_roles' AND column_name = 'description') THEN
    ALTER TABLE user_roles ADD COLUMN description TEXT;
  END IF;
END $$;

-- Update existing roles with descriptions
UPDATE user_roles SET description = 'Full system access' WHERE name = 'admin' AND description IS NULL;
UPDATE user_roles SET description = 'Basic inventory management' WHERE name = 'staff' AND description IS NULL;

-- Add new roles if they don't exist
INSERT INTO user_roles (name, description) VALUES 
('manager', 'Department manager with approval rights'),
('caretaker', 'Responsible for maintenance and condition reporting'),
('borrower', 'Can only view and request to borrow items'),
('procurement_officer', 'Can manage procurement requests')
ON CONFLICT (name) DO NOTHING;

-- Create role permissions table
CREATE TABLE IF NOT EXISTS permissions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name VARCHAR(64) NOT NULL UNIQUE,
  description TEXT
);

-- Insert basic permissions
INSERT INTO permissions (name, description) VALUES
('view_items', 'Can view inventory items'),
('add_items', 'Can add new inventory items'),
('edit_items', 'Can edit existing inventory items'),
('delete_items', 'Can delete inventory items'),
('manage_categories', 'Can manage item categories'),
('manage_locations', 'Can manage locations'),
('manage_users', 'Can manage user accounts'),
('approve_procurements', 'Can approve procurement requests'),
('manage_donations', 'Can manage donations'),
('view_reports', 'Can view system reports'),
('manage_item_status', 'Can change item status'),
('borrow_items', 'Can borrow items'),
('admin_access', 'Full administrative access'),
('manage_permissions', 'Can manage role permissions'),
('manage_roles', 'Can manage user roles'),
('approve_borrowings', 'Can approve item borrowing requests'),
('manage_borrowings', 'Can manage all borrowings'),
('view_all_borrowings', 'Can view all borrowings')
ON CONFLICT (name) DO NOTHING;

-- Create role-permission mapping table
CREATE TABLE IF NOT EXISTS role_permissions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  role_id UUID REFERENCES user_roles(id) ON DELETE CASCADE,
  permission_id UUID REFERENCES permissions(id) ON DELETE CASCADE,
  UNIQUE(role_id, permission_id)
);

-- Assign permissions to roles if they don't already exist
-- Admin role (all permissions)
DO $$ 
BEGIN
  -- First, clear existing permissions for the admin role to avoid duplicates
  DELETE FROM role_permissions
  WHERE role_id = (SELECT id FROM user_roles WHERE name = 'admin');
  
  -- Then assign all permissions to admin
  INSERT INTO role_permissions (role_id, permission_id)
  SELECT 
    (SELECT id FROM user_roles WHERE name = 'admin'),
    id
  FROM permissions;
END $$;

-- Staff role
DO $$ 
BEGIN
  -- First, clear existing permissions for this role to avoid duplicates
  DELETE FROM role_permissions
  WHERE role_id = (SELECT id FROM user_roles WHERE name = 'staff');
  
  -- Then assign specific permissions
  INSERT INTO role_permissions (role_id, permission_id)
  SELECT 
    (SELECT id FROM user_roles WHERE name = 'staff'),
    id
  FROM permissions
  WHERE name IN ('view_items', 'add_items', 'edit_items', 'view_reports', 'manage_item_status', 'borrow_items');
END $$;

-- Manager role
DO $$ 
BEGIN
  -- Only proceed if the role exists
  IF EXISTS (SELECT 1 FROM user_roles WHERE name = 'manager') THEN
    -- Clear existing permissions for this role to avoid duplicates
    DELETE FROM role_permissions
    WHERE role_id = (SELECT id FROM user_roles WHERE name = 'manager');
    
    -- Then assign specific permissions
    INSERT INTO role_permissions (role_id, permission_id)
    SELECT 
      (SELECT id FROM user_roles WHERE name = 'manager'),
      id
    FROM permissions
    WHERE name IN ('view_items', 'add_items', 'edit_items', 'delete_items', 'manage_categories', 
                 'manage_locations', 'approve_procurements', 'view_reports', 'manage_item_status',
                 'approve_borrowings', 'view_all_borrowings');
  END IF;
END $$;

-- Caretaker role
DO $$ 
BEGIN
  -- Only proceed if the role exists
  IF EXISTS (SELECT 1 FROM user_roles WHERE name = 'caretaker') THEN
    -- Clear existing permissions for this role to avoid duplicates
    DELETE FROM role_permissions
    WHERE role_id = (SELECT id FROM user_roles WHERE name = 'caretaker');
    
    -- Then assign specific permissions
    INSERT INTO role_permissions (role_id, permission_id)
    SELECT 
      (SELECT id FROM user_roles WHERE name = 'caretaker'),
      id
    FROM permissions
    WHERE name IN ('view_items', 'edit_items', 'manage_item_status');
  END IF;
END $$;

-- Borrower role
DO $$ 
BEGIN
  -- Only proceed if the role exists
  IF EXISTS (SELECT 1 FROM user_roles WHERE name = 'borrower') THEN
    -- Clear existing permissions for this role to avoid duplicates
    DELETE FROM role_permissions
    WHERE role_id = (SELECT id FROM user_roles WHERE name = 'borrower');
    
    -- Then assign specific permissions
    INSERT INTO role_permissions (role_id, permission_id)
    SELECT 
      (SELECT id FROM user_roles WHERE name = 'borrower'),
      id
    FROM permissions
    WHERE name IN ('view_items', 'borrow_items');
  END IF;
END $$;

-- Procurement officer role
DO $$ 
BEGIN
  -- Only proceed if the role exists
  IF EXISTS (SELECT 1 FROM user_roles WHERE name = 'procurement_officer') THEN
    -- Clear existing permissions for this role to avoid duplicates
    DELETE FROM role_permissions
    WHERE role_id = (SELECT id FROM user_roles WHERE name = 'procurement_officer');
    
    -- Then assign specific permissions
    INSERT INTO role_permissions (role_id, permission_id)
    SELECT 
      (SELECT id FROM user_roles WHERE name = 'procurement_officer'),
      id
    FROM permissions
    WHERE name IN ('view_items', 'add_items', 'manage_categories', 'approve_procurements', 'view_reports');
  END IF;
END $$;

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

-- Create indexes for better performance if they don't exist
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
