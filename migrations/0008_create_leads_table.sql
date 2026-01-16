-- Create leads table
CREATE TABLE IF NOT EXISTS leads (
    id VARCHAR(50) PRIMARY KEY,
    first_name VARCHAR(100) NOT NULL,
    last_name VARCHAR(100) NOT NULL,
    email VARCHAR(255) NOT NULL,
    phone VARCHAR(20),
    company VARCHAR(255) NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'New',
    source VARCHAR(100) NOT NULL,
    title VARCHAR(100),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Create index on email for faster lookups
CREATE INDEX IF NOT EXISTS idx_leads_email ON leads(email);

-- Create index on company for faster searches
CREATE INDEX IF NOT EXISTS idx_leads_company ON leads(company);

-- Create index on status for filtering
CREATE INDEX IF NOT EXISTS idx_leads_status ON leads(status);

-- Create index on source for filtering
CREATE INDEX IF NOT EXISTS idx_leads_source ON leads(source);

-- Create trigger to update updated_at column (function already exists from previous migration)
CREATE TRIGGER update_leads_updated_at 
    BEFORE UPDATE ON leads 
    FOR EACH ROW 
    EXECUTE FUNCTION update_updated_at_column();