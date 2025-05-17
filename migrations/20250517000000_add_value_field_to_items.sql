-- Add value field to items table
ALTER TABLE items ADD COLUMN IF NOT EXISTS value TEXT;

-- Update existing functions to include value field
CREATE OR REPLACE FUNCTION create_item(
    p_name TEXT,
    p_category_id UUID,
    p_quantity INTEGER,
    p_condition_id UUID,
    p_location_id UUID,
    p_photo_url TEXT,
    p_source_id UUID,
    p_donor_id UUID,
    p_procurement_id UUID,
    p_status_id UUID,
    p_value TEXT
) RETURNS items AS $$
DECLARE
    new_item items;
BEGIN
    INSERT INTO items (
        name, 
        category_id, 
        quantity, 
        condition_id, 
        location_id, 
        photo_url, 
        source_id, 
        donor_id, 
        procurement_id, 
        status_id, 
        value
    ) VALUES (
        p_name, 
        p_category_id, 
        p_quantity, 
        p_condition_id, 
        p_location_id, 
        p_photo_url, 
        p_source_id, 
        p_donor_id, 
        p_procurement_id, 
        p_status_id, 
        p_value
    ) RETURNING * INTO new_item;
    
    RETURN new_item;
END;
$$ LANGUAGE plpgsql;

-- Update function for updating items
CREATE OR REPLACE FUNCTION update_item(
    p_id UUID,
    p_name TEXT,
    p_category_id UUID,
    p_quantity INTEGER,
    p_condition_id UUID,
    p_location_id UUID,
    p_photo_url TEXT,
    p_source_id UUID,
    p_donor_id UUID,
    p_procurement_id UUID,
    p_status_id UUID,
    p_value TEXT
) RETURNS items AS $$
DECLARE
    updated_item items;
BEGIN
    UPDATE items
    SET 
        name = COALESCE(p_name, name),
        category_id = COALESCE(p_category_id, category_id),
        quantity = COALESCE(p_quantity, quantity),
        condition_id = COALESCE(p_condition_id, condition_id),
        location_id = p_location_id,
        photo_url = p_photo_url,
        source_id = COALESCE(p_source_id, source_id),
        donor_id = p_donor_id,
        procurement_id = p_procurement_id,
        status_id = COALESCE(p_status_id, status_id),
        value = p_value
    WHERE id = p_id
    RETURNING * INTO updated_item;
    
    RETURN updated_item;
END;
$$ LANGUAGE plpgsql;
