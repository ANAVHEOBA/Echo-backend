-- Create webhook_subscriptions table for managing user webhook subscriptions
CREATE TABLE IF NOT EXISTS webhook_subscriptions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    platform VARCHAR(50) NOT NULL, -- 'slack', 'gmail', 'zoom', 'generic'
    webhook_url VARCHAR(500) NOT NULL,
    secret VARCHAR(255) NOT NULL, -- For signature validation
    event_types JSONB, -- Array of event types to subscribe to
    active BOOLEAN DEFAULT true,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Create indexes
CREATE INDEX IF NOT EXISTS idx_webhook_subscriptions_user_id ON webhook_subscriptions(user_id);
CREATE INDEX IF NOT EXISTS idx_webhook_subscriptions_platform ON webhook_subscriptions(platform);
CREATE INDEX IF NOT EXISTS idx_webhook_subscriptions_active ON webhook_subscriptions(active);

-- Trigger to update updated_at
CREATE TRIGGER update_webhook_subscriptions_updated_at 
    BEFORE UPDATE ON webhook_subscriptions 
    FOR EACH ROW 
    EXECUTE FUNCTION update_updated_at_column();
