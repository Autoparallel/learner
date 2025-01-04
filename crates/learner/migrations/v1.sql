PRAGMA foreign_keys = ON;

-- Version tracking
CREATE TABLE IF NOT EXISTS schema_version (
    version INTEGER PRIMARY KEY NOT NULL,
    applied_at TEXT NOT NULL DEFAULT (datetime('now'))
) STRICT;

-- Main resource storage with JSON documents
CREATE TABLE IF NOT EXISTS resources (
    id INTEGER PRIMARY KEY,
    identifier TEXT NOT NULL,        -- Unique identifier for the resource
    resource_type TEXT NOT NULL,     -- Type of resource (e.g. "paper")
    -- The complete record data
    resource JSON NOT NULL,          -- The Resource part
    state JSON NOT NULL,             -- The State part
    storage JSON NOT NULL,           -- The Storage part
    retrieval JSON NOT NULL,         -- The Retrieval part
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    -- Ensure each resource has a unique identifier within its type
    UNIQUE(resource_type, identifier)
) STRICT;

-- Helper table for JSON extraction
CREATE TABLE IF NOT EXISTS json_tree_helpers (
    value TEXT
);

-- Full-text search
CREATE VIRTUAL TABLE IF NOT EXISTS resources_fts USING fts5(
    searchable_text,
    content=resources,
    content_rowid=id,
    tokenize='unicode61 remove_diacritics 1'
);

-- Extract searchable text from JSON before insert
CREATE TRIGGER resources_before_insert BEFORE INSERT ON resources BEGIN
    DELETE FROM json_tree_helpers;
    
    WITH RECURSIVE json_extract_strings(value) AS (
        SELECT value 
        FROM json_tree(NEW.resource)
        WHERE type = 'text'
        UNION ALL
        SELECT value 
        FROM json_tree(NEW.state)
        WHERE type = 'text'
        UNION ALL
        SELECT value 
        FROM json_tree(NEW.storage)
        WHERE type = 'text'
        UNION ALL
        SELECT value 
        FROM json_tree(NEW.retrieval)
        WHERE type = 'text'
    )
    INSERT INTO json_tree_helpers 
    SELECT value FROM json_extract_strings WHERE value IS NOT NULL;
END;

-- Update FTS index after insert
CREATE TRIGGER resources_ai AFTER INSERT ON resources BEGIN
    INSERT INTO resources_fts(rowid, searchable_text)
    VALUES (
        new.id,
        (SELECT group_concat(value, ' ') FROM json_tree_helpers)
    );
END;

-- Keep FTS updated
CREATE TRIGGER resources_au AFTER UPDATE ON resources BEGIN
    INSERT INTO resources_fts(resources_fts, rowid, searchable_text)
    VALUES(
        'delete',
        old.id,
        NULL
    );
    INSERT INTO resources_fts(rowid, searchable_text)
    VALUES (
        new.id,
        (SELECT group_concat(value, ' ') FROM json_tree_helpers)
    );
END;

-- Clean up FTS when deleting
CREATE TRIGGER resources_ad AFTER DELETE ON resources BEGIN
    INSERT INTO resources_fts(resources_fts, rowid, searchable_text)
    VALUES('delete', old.id, NULL);
END;

-- Set initial version
INSERT INTO schema_version (version) VALUES (1);