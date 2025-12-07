-- Add customOpenAIConfig column to settings table
-- This stores custom OpenAI-compatible endpoint configuration as JSON
ALTER TABLE settings ADD COLUMN customOpenAIConfig TEXT;
