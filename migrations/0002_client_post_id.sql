ALTER TABLE posts ADD COLUMN IF NOT EXISTS client_post_id UUID;
ALTER TABLE posts ADD COLUMN IF NOT EXISTS create_revision BIGINT;
CREATE UNIQUE INDEX IF NOT EXISTS posts_actor_client_post_uidx
    ON posts (actor_id, client_post_id) WHERE client_post_id IS NOT NULL;
