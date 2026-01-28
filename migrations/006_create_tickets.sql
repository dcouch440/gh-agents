-- Tickets table
CREATE TABLE IF NOT EXISTS tickets (
    id TEXT PRIMARY KEY NOT NULL,
    source_type TEXT NOT NULL DEFAULT 'manual',
    source_owner TEXT,
    source_repo TEXT,
    source_issue_number INTEGER,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    labels TEXT NOT NULL DEFAULT '[]',
    status TEXT NOT NULL DEFAULT 'new',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Vertical slices table
CREATE TABLE IF NOT EXISTS vertical_slices (
    id TEXT PRIMARY KEY NOT NULL,
    ticket_id TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),

    FOREIGN KEY (ticket_id) REFERENCES tickets(id)
);

-- Index for ticket queries
CREATE INDEX IF NOT EXISTS idx_tickets_status ON tickets(status);
CREATE INDEX IF NOT EXISTS idx_slices_ticket ON vertical_slices(ticket_id);
CREATE INDEX IF NOT EXISTS idx_slices_status ON vertical_slices(status);
