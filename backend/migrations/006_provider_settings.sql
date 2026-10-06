ALTER TABLE studio.ai_budget ADD COLUMN provider text NOT NULL DEFAULT 'openrouter' CHECK(provider IN ('openrouter','typesafe'));
