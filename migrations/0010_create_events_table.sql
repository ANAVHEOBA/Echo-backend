-- Create events table for storing all incoming webhook events
CREATE TABLE IF NOT EXISTS events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_type VARCHAR(100) NOT NULL,
    source VARCHAR(50) NOT NULL, -- 'slack', 'gmail', 'zoom', 'salesforce', 'generic'
    external_id VARCHAR(255), -- ID from source system for deduplication
    payload JSONB NOT NULL,
    processed_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    
    -- Deduplication constraint
    UNIQUE(source, external_id)
);

-- Create indexes for common queries
CREATE INDEX IF NOT EXISTS idx_events_event_type ON events(event_type);
CREATE INDEX IF NOT EXISTS idx_events_source ON events(source);
CREATE INDEX IF NOT EXISTS idx_events_processed_at ON events(processed_at);
CREATE INDEX IF NOT EXISTS idx_events_created_at ON events(created_at DESC);

-- Index for unprocessed events
CREATE INDEX IF NOT EXISTS idx_events_unprocessed ON events(processed_at) WHERE processed_at IS NULL;

-- Trigger to update updated_at
CREATE TRIGGER update_events_updated_at 
    BEFORE UPDATE ON events 
    FOR EACH ROW 
    EXECUTE FUNCTION update_updated_at_column();
