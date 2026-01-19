-- Add migration script here
CREATE TYPE room_role AS ENUM ('player', 'spectator');

CREATE TABLE room_participants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    room_id UUID NOT NULL REFERENCES rooms(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role room_role NOT NULL,
    position INT, -- 1 or 2 for players, NULL for spectators
    joined_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE (room_id, user_id)
);

-- Only 2 players max (enforced at app-level mostly)
CREATE UNIQUE INDEX idx_room_player_position
ON room_participants(room_id, position)
WHERE role = 'player';
