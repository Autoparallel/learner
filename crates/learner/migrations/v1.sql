PRAGMA foreign_keys = ON;

-- Version tracking
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER PRIMARY KEY NOT NULL,
    applied_at TEXT NOT NULL DEFAULT (datetime('now'))
) STRICT;

-- Core resource storage
CREATE TABLE IF NOT EXISTS resources (
    id INTEGER PRIMARY KEY,
    type TEXT NOT NULL,           -- Resource type identifier (e.g., "paper", "book")
    title TEXT,                   -- Denormalized for common queries
    metadata JSON NOT NULL,       -- Complete resource data
    searchable_text TEXT,         -- For full-text search
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
) STRICT;

-- Resource state tracking
CREATE TABLE IF NOT EXISTS resource_states (
    resource_id INTEGER PRIMARY KEY,
    read_status TEXT NOT NULL DEFAULT 'unread',      -- 'unread', 'reading', 'completed'
    rating INTEGER CHECK (rating BETWEEN 1 AND 5),   -- Optional 1-5 rating
    starred BOOLEAN NOT NULL DEFAULT 0,
    last_accessed TEXT,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (resource_id) REFERENCES resources(id) ON DELETE CASCADE
) STRICT;

-- Tag management
CREATE TABLE IF NOT EXISTS tags (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
) STRICT;

CREATE TABLE IF NOT EXISTS resource_tags (
    resource_id INTEGER NOT NULL,
    tag_id INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (resource_id, tag_id),
    FOREIGN KEY (resource_id) REFERENCES resources(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
) STRICT;

-- Full-text search
CREATE VIRTUAL TABLE IF NOT EXISTS resources_fts USING fts5(
    title,
    searchable_text,
    content=resources,
    content_rowid=id,
    tokenize='unicode61 remove_diacritics 1'
);

-- FTS triggers
CREATE TRIGGER resources_ai AFTER INSERT ON resources BEGIN
    INSERT INTO resources_fts(rowid, title, searchable_text)
    VALUES (new.id, new.title, new.searchable_text);
END;

-- Set initial version
INSERT INTO schema_version (version) VALUES (1);