-- Seal system files: pinned steps mark their artifacts as immutable.
ALTER TABLE system_files ADD COLUMN sealed BOOLEAN NOT NULL DEFAULT false;
