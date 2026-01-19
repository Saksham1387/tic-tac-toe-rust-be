-- Add migration script here
CREATE TYPE room_status AS ENUM ('waiting', 'in_progress', 'completed');

CREATE TABLE rooms (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    room_name VARCHAR(255) NOT NULL,
    room_code VARCHAR(10) UNIQUE NOT NULL,
    is_private BOOLEAN DEFAULT false,
    max_players INT NOT NULL DEFAULT 2,
    max_spectators INT DEFAULT 0,
    status room_status DEFAULT 'waiting',
    created_by UUID REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX idx_rooms_status ON rooms(status);
CREATE INDEX idx_rooms_created_at ON rooms(created_at DESC);
