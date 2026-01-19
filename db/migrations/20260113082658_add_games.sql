-- Add migration script here
CREATE TYPE game_status AS ENUM ('in_progress', 'completed');

CREATE TABLE games (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    room_id UUID REFERENCES rooms(id) ON DELETE SET NULL,
    player1_id UUID REFERENCES users(id),
    player2_id UUID REFERENCES users(id),
    winner_id UUID REFERENCES users(id),
    status game_status DEFAULT 'in_progress',
    total_moves INT DEFAULT 0,
    duration_seconds INT,
    completed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_games_player1 ON games(player1_id);
CREATE INDEX idx_games_player2 ON games(player2_id);
CREATE INDEX idx_games_winner ON games(winner_id);
CREATE INDEX idx_games_completed_at ON games(completed_at DESC);
