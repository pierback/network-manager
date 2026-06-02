PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS schema_migrations (
    version INTEGER PRIMARY KEY,
    applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS daemon_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS device_identities (
    id TEXT PRIMARY KEY,
    stable_key TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS device_user_intent (
    identity_id TEXT PRIMARY KEY REFERENCES device_identities(id) ON DELETE CASCADE,
    tracked_state TEXT NOT NULL DEFAULT 'untracked' CHECK (tracked_state IN ('untracked', 'tracked', 'ignored')),
    label TEXT,
    alias TEXT UNIQUE,
    category TEXT,
    ssh_username TEXT,
    ssh_port INTEGER CHECK (ssh_port IS NULL OR (ssh_port > 0 AND ssh_port <= 65535)),
    endpoint_preference TEXT CHECK (endpoint_preference IS NULL OR endpoint_preference IN ('auto', 'local_first', 'tailscale_first', 'lan_first')),
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS device_tags (
    identity_id TEXT NOT NULL REFERENCES device_identities(id) ON DELETE CASCADE,
    tag TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (identity_id, tag)
);

CREATE TABLE IF NOT EXISTS discovered_devices (
    id TEXT PRIMARY KEY,
    source TEXT NOT NULL,
    source_device_id TEXT NOT NULL,
    display_name TEXT,
    raw_json TEXT NOT NULL DEFAULT '{}',
    first_seen_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    last_seen_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (source, source_device_id)
);

CREATE TABLE IF NOT EXISTS discovery_observations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    discovered_device_id TEXT NOT NULL REFERENCES discovered_devices(id) ON DELETE CASCADE,
    identity_id TEXT REFERENCES device_identities(id) ON DELETE SET NULL,
    source TEXT NOT NULL,
    interface_name TEXT,
    observed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    evidence_json TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS network_endpoints (
    id TEXT PRIMARY KEY,
    identity_id TEXT NOT NULL REFERENCES device_identities(id) ON DELETE CASCADE,
    kind TEXT NOT NULL CHECK (kind IN ('lan_dns', 'mdns', 'lan_ip', 'tailscale_dns', 'tailscale_ip', 'other')),
    address TEXT NOT NULL,
    port INTEGER CHECK (port IS NULL OR (port > 0 AND port <= 65535)),
    hostname TEXT,
    source TEXT NOT NULL,
    interface_name TEXT,
    reachable_state TEXT NOT NULL DEFAULT 'unknown' CHECK (reachable_state IN ('online', 'offline', 'unknown')),
    ssh_capability_state TEXT NOT NULL DEFAULT 'unknown' CHECK (ssh_capability_state IN ('online', 'offline', 'unknown')),
    last_seen_at TEXT,
    last_checked_at TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS identity_evidence (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    identity_id TEXT REFERENCES device_identities(id) ON DELETE CASCADE,
    discovered_device_id TEXT REFERENCES discovered_devices(id) ON DELETE CASCADE,
    evidence_type TEXT NOT NULL,
    evidence_value TEXT NOT NULL,
    confidence REAL NOT NULL CHECK (confidence >= 0.0 AND confidence <= 1.0),
    source TEXT NOT NULL,
    observed_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS identity_corrections (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    correction_type TEXT NOT NULL CHECK (correction_type IN ('merge', 'split')),
    from_identity_id TEXT NOT NULL REFERENCES device_identities(id) ON DELETE CASCADE,
    to_identity_id TEXT REFERENCES device_identities(id) ON DELETE CASCADE,
    reason TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_device_user_intent_alias ON device_user_intent(alias);
CREATE INDEX IF NOT EXISTS idx_device_user_intent_state ON device_user_intent(tracked_state);
CREATE INDEX IF NOT EXISTS idx_discovered_devices_source ON discovered_devices(source, source_device_id);
CREATE INDEX IF NOT EXISTS idx_observations_identity ON discovery_observations(identity_id, observed_at);
CREATE INDEX IF NOT EXISTS idx_observations_device ON discovery_observations(discovered_device_id, observed_at);
CREATE UNIQUE INDEX IF NOT EXISTS idx_endpoints_unique ON network_endpoints(identity_id, kind, address, COALESCE(port, 0));
CREATE INDEX IF NOT EXISTS idx_endpoints_identity ON network_endpoints(identity_id);
CREATE INDEX IF NOT EXISTS idx_endpoints_kind ON network_endpoints(kind);
CREATE INDEX IF NOT EXISTS idx_identity_evidence_identity ON identity_evidence(identity_id);
CREATE INDEX IF NOT EXISTS idx_identity_corrections_from ON identity_corrections(from_identity_id);

INSERT OR IGNORE INTO schema_migrations(version) VALUES (1);
