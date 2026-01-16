-- Create opportunities table
CREATE TABLE IF NOT EXISTS opportunities (
    id VARCHAR(50) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    amount NUMERIC(15, 2),
    stage VARCHAR(100) NOT NULL DEFAULT 'New',
    probability INTEGER CHECK (probability >= 0 AND probability <= 100),
    close_date DATE,
    contact_id VARCHAR(50) NOT NULL,
    description TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (contact_id) REFERENCES contacts(id) ON DELETE CASCADE
);

-- Create index on contact_id for faster lookups
CREATE INDEX IF NOT EXISTS idx_opportunities_contact_id ON opportunities(contact_id);

-- Create index on stage for filtering
CREATE INDEX IF NOT EXISTS idx_opportunities_stage ON opportunities(stage);

-- Create index on close_date for filtering
CREATE INDEX IF NOT EXISTS idx_opportunities_close_date ON opportunities(close_date);

-- Create trigger to update updated_at column (function already exists from previous migration)
CREATE TRIGGER update_opportunities_updated_at 
    BEFORE UPDATE ON opportunities 
    FOR EACH ROW 
    EXECUTE FUNCTION update_updated_at_column();
