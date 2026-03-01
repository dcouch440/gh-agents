-- Per-step image cache. Stores pre-rendered stroke PNGs (base64) at board-submit
-- time so execution reads the image directly instead of re-rasterizing from lossy
-- encoded coordinates.

CREATE TABLE step_images (
    step_id UUID PRIMARY KEY REFERENCES workflow_steps(id) ON DELETE CASCADE,
    stroke_image_base64 TEXT NOT NULL DEFAULT '',
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
