-- Create a separate database for JuiceFS metadata.
-- JuiceFS stores filesystem metadata (inodes, chunks, etc.) here.
-- Kept separate from the nexor app database to avoid table conflicts.
CREATE DATABASE juicefs;
