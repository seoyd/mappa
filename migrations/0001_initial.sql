CREATE EXTENSION IF NOT EXISTS postgis;

CREATE TABLE IF NOT EXISTS posts (
    id UUID PRIMARY KEY,
    actor_id UUID NOT NULL,
    cell_id BIGINT NOT NULL CHECK (cell_id >= 0),
    lat_e7 INTEGER NOT NULL CHECK (lat_e7 BETWEEN -900000000 AND 900000000),
    lon_e7 INTEGER NOT NULL CHECK (lon_e7 BETWEEN -1800000000 AND 1800000000),
    geom geography(Point, 4326) NOT NULL,
    kind SMALLINT NOT NULL CHECK (kind = 1),
    body TEXT NOT NULL CHECK (octet_length(body) BETWEEN 1 AND 8192),
    created_at BIGINT NOT NULL,
    expires_at BIGINT
);
CREATE INDEX IF NOT EXISTS posts_cell_id_idx ON posts(cell_id);
CREATE INDEX IF NOT EXISTS posts_cell_created_idx ON posts(cell_id, created_at);
CREATE INDEX IF NOT EXISTS posts_geom_gist_idx ON posts USING GIST(geom);

CREATE TABLE IF NOT EXISTS cell_state (
    cell_id BIGINT PRIMARY KEY CHECK (cell_id >= 0),
    revision BIGINT NOT NULL CHECK (revision >= 1),
    updated_at BIGINT NOT NULL
);
