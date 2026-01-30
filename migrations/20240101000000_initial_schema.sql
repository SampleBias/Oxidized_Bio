-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
-- pgvector extension is named 'vector' in the pg_extensions catalog
CREATE EXTENSION IF NOT EXISTS "vector";

-- Drop existing tables (cascade to handle foreign keys)
DROP TABLE IF EXISTS x402_external CASCADE;
DROP TABLE IF EXISTS x402_payments CASCADE;
DROP TABLE IF EXISTS messages CASCADE;
DROP TABLE IF EXISTS states CASCADE;
DROP TABLE IF EXISTS conversations CASCADE;
DROP TABLE IF EXISTS conversation_states CASCADE;
DROP TABLE IF EXISTS users CASCADE;

-- Users table
CREATE TABLE users (
  id UUID DEFAULT gen_random_uuid() PRIMARY KEY,
  username TEXT NOT NULL UNIQUE,
  email TEXT NOT NULL UNIQUE,
  wallet_address TEXT UNIQUE,
  used_invite_code TEXT,
  points INTEGER DEFAULT 0,
  has_completed_invite_flow BOOLEAN DEFAULT false,
  invite_codes_remaining INTEGER DEFAULT 0,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Index for fast wallet lookups
CREATE INDEX idx_users_wallet_address ON users(wallet_address);

-- Conversation States table (stores persistent conversation state)
CREATE TABLE conversation_states (
  id UUID DEFAULT gen_random_uuid() PRIMARY KEY,
  values JSONB NOT NULL DEFAULT '{}',
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Conversations table
CREATE TABLE conversations (
  id UUID DEFAULT gen_random_uuid() PRIMARY KEY,
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  conversation_state_id UUID REFERENCES conversation_states(id) ON DELETE SET NULL,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- States table (stores message processing state)
CREATE TABLE states (
  id UUID DEFAULT gen_random_uuid() PRIMARY KEY,
  values JSONB NOT NULL DEFAULT '{}',
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Messages table
CREATE TABLE messages (
  id UUID DEFAULT gen_random_uuid() PRIMARY KEY,
  conversation_id UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  question TEXT,
  content TEXT NOT NULL,
  state_id UUID REFERENCES states(id) ON DELETE SET NULL,
  response_time INTEGER,
  source TEXT DEFAULT 'ui',
  files JSONB,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create indexes for common queries
CREATE INDEX idx_conversations_user_id ON conversations(user_id);
CREATE INDEX idx_conversations_created_at ON conversations(created_at DESC);
CREATE INDEX idx_conversations_conversation_state_id ON conversations(conversation_state_id);

CREATE INDEX idx_messages_conversation_id ON messages(conversation_id);
CREATE INDEX idx_messages_user_id ON messages(user_id);
CREATE INDEX idx_messages_created_at ON messages(created_at DESC);
CREATE INDEX idx_messages_state_id ON messages(state_id);

-- GIN index for JSONB fields (efficient for JSON queries)
CREATE INDEX idx_states_values ON states USING GIN (values);
CREATE INDEX idx_conversation_states_values ON conversation_states USING GIN (values);
CREATE INDEX idx_messages_files ON messages USING GIN (files);

-- Function to automatically update updated_at timestamp
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
  NEW.updated_at = NOW();
  RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Triggers to update updated_at on record updates
CREATE TRIGGER update_users_updated_at
  BEFORE UPDATE ON users
  FOR EACH ROW
  EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_conversations_updated_at
  BEFORE UPDATE ON conversations
  FOR EACH ROW
  EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_states_updated_at
  BEFORE UPDATE ON states
  FOR EACH ROW
  EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_conversation_states_updated_at
  BEFORE UPDATE ON conversation_states
  FOR EACH ROW
  EXECUTE FUNCTION update_updated_at_column();

-- x402 Payment records table
CREATE TABLE x402_payments (
  id UUID DEFAULT gen_random_uuid() PRIMARY KEY,
  user_id UUID REFERENCES users(id) ON DELETE SET NULL,
  conversation_id UUID REFERENCES conversations(id) ON DELETE SET NULL,
  message_id UUID REFERENCES messages(id) ON DELETE SET NULL,
  amount_usd NUMERIC NOT NULL,
  amount_wei TEXT NOT NULL,
  asset TEXT NOT NULL DEFAULT 'USDC',
  network TEXT NOT NULL,
  tools_used TEXT[],
  tx_hash TEXT,
  network_id TEXT,
  payment_status TEXT NOT NULL CHECK (payment_status IN ('pending', 'verified', 'settled', 'failed')),
  payment_header JSONB,
  payment_requirements JSONB,
  error_message TEXT,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
  verified_at TIMESTAMP WITH TIME ZONE,
  settled_at TIMESTAMP WITH TIME ZONE
);

-- Indexes for x402_payments
CREATE INDEX idx_x402_payments_user_id ON x402_payments(user_id);
CREATE INDEX idx_x402_payments_conversation_id ON x402_payments(conversation_id);
CREATE INDEX idx_x402_payments_message_id ON x402_payments(message_id);
CREATE INDEX idx_x402_payments_tx_hash ON x402_payments(tx_hash);
CREATE INDEX idx_x402_payments_status ON x402_payments(payment_status);
CREATE INDEX idx_x402_payments_created_at ON x402_payments(created_at DESC);

-- GIN indexes for JSONB fields
CREATE INDEX idx_x402_payments_payment_header ON x402_payments USING GIN (payment_header);
CREATE INDEX idx_x402_payments_payment_requirements ON x402_payments USING GIN (payment_requirements);

-- x402 External Requests table (for external API consumers)
CREATE TABLE x402_external (
  id UUID DEFAULT gen_random_uuid() PRIMARY KEY,
  conversation_id UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
  request_path TEXT NOT NULL,
  tx_hash TEXT,
  amount_usd NUMERIC,
  amount_wei TEXT,
  asset TEXT DEFAULT 'USDC',
  network TEXT,
  network_id TEXT,
  payment_status TEXT CHECK (payment_status IN ('pending', 'verified', 'settled', 'failed')),
  payment_header JSONB,
  payment_requirements JSONB,
  request_metadata JSONB,
  response_time INTEGER,
  error_message TEXT,
  created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);
