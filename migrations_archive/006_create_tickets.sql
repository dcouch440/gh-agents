-- Tickets table
CREATE TABLE IF NOT EXISTS tickets (
    id UUID PRIMARY KEY NOT NULL,
    source_type TEXT NOT NULL DEFAULT 'manual',
    source_owner TEXT,
    source_repo TEXT,
    source_issue_number INTEGER,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    labels JSONB NOT NULL DEFAULT '[]',
    status TEXT NOT NULL DEFAULT 'new',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Vertical slices table
CREATE TABLE IF NOT EXISTS vertical_slices (
    id UUID PRIMARY KEY NOT NULL,
    ticket_id UUID NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    FOREIGN KEY (ticket_id) REFERENCES tickets(id)
);

-- Index for ticket queries
CREATE INDEX IF NOT EXISTS idx_tickets_status ON tickets(status);
CREATE INDEX IF NOT EXISTS idx_slices_ticket ON vertical_slices(ticket_id);
CREATE INDEX IF NOT EXISTS idx_slices_status ON vertical_slices(status);
