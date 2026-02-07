-- Security Hardening Phase 1: Add admin role to users
ALTER TABLE public.users ADD COLUMN is_admin boolean NOT NULL DEFAULT false;
