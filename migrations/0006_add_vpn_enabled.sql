-- Add VPN configuration to workflows.
-- When vpn_enabled = true AND container_enabled = true, each agent container
-- gets a unique WireGuard VPN peer and runs behind a VPN sidecar.

ALTER TABLE public.workflows
    ADD COLUMN vpn_enabled BOOLEAN NOT NULL DEFAULT false;
