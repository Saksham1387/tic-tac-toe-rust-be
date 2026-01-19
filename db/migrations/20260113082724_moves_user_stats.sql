-- Add migration script here
CREATE TABLE game_moves (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    game_id UUID REFERENCES games(id) ON DELETE CASCADE,
    player_id UUID REFERENCES users(id),
    position INT NOT NULL CHECK (position BETWEEN 0 AND 8),
    symbol CHAR(1) NOT NULL CHECK (symbol IN ('X', 'O')),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE (game_id, position)
);

CREATE TABLE user_stats (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    total_games INT DEFAULT 0,
    wins INT DEFAULT 0,
    losses INT DEFAULT 0,
    draws INT DEFAULT 0,
    current_streak INT DEFAULT 0,
    longest_streak INT DEFAULT 0,
    updated_at TIMESTAMPTZ DEFAULT NOW()
);
