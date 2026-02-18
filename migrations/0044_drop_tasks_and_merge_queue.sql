-- Drop legacy tables: tasks (unused standalone task entity) and pr_merge_queue (unused merge queue)
DROP TABLE IF EXISTS tasks;
DROP TABLE IF EXISTS pr_merge_queue;
