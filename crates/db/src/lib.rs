use anyhow::{bail, Context, Result};
use network_manager_core::{
    AvailabilityState, DeviceIdentity, DiscoveredDevice, EndpointKind, EndpointPreference,
    NetworkEndpoint, TrackedState,
};
use rusqlite::types::Type;
use rusqlite::{params, Connection, OptionalExtension};
use std::error::Error;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use uuid::Uuid;

const INITIAL_MIGRATION: &str = include_str!("../migrations/0001_initial.sql");

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DaemonStatus {
    pub state: String,
    pub source: String,
    pub started_at: Option<String>,
    pub updated_at: Option<String>,
    pub db_path: Option<String>,
    pub stale: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeviceIdentityRecord {
    pub identity: DeviceIdentity,
    pub endpoint_count: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiscoveredDeviceRecord {
    pub device: DiscoveredDevice,
    pub identity_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TailscaleNodeObservation {
    pub source_device_id: String,
    pub display_name: Option<String>,
    pub dns_name: Option<String>,
    pub tailscale_ips: Vec<String>,
    pub online: Option<bool>,
    pub os: Option<String>,
    pub raw_json: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LanDeviceObservation {
    pub ip_address: String,
    pub hostname: Option<String>,
    pub mac_address: Option<String>,
    pub interface_name: Option<String>,
    pub raw_text: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MdnsServiceObservation {
    pub source_device_id: String,
    pub service_name: String,
    pub service_type: String,
    pub domain: String,
    pub hostname: Option<String>,
    pub ip_addresses: Vec<String>,
    pub port: Option<u16>,
    pub raw_text: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeviceDetails {
    pub identity: DeviceIdentity,
    pub endpoints: Vec<NetworkEndpoint>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeviceMutationResult {
    pub identity: DeviceIdentity,
    pub endpoint_count: usize,
    pub message: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IdentityCorrectionResult {
    pub identity_id: String,
    pub affected_identity_id: String,
    pub message: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserSettingsExport {
    pub format_version: u32,
    pub devices: Vec<DeviceSettingsExport>,
    pub merges: Vec<MergeSettingsExport>,
    pub splits: Vec<SplitSettingsExport>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeviceSettingsExport {
    pub stable_key: String,
    pub tracked_state: TrackedState,
    pub label: Option<String>,
    pub alias: Option<String>,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub ssh_username: Option<String>,
    pub ssh_port: Option<u16>,
    pub endpoint_preference: EndpointPreference,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MergeSettingsExport {
    pub source_stable_key: String,
    pub target_stable_key: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SplitSettingsExport {
    pub source: String,
    pub source_device_id: String,
    pub target_stable_key: String,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserSettingsImportResult {
    pub dry_run: bool,
    pub devices_applied: usize,
    pub devices_missing: usize,
    pub merges_applied: usize,
    pub merges_skipped: usize,
    pub splits_applied: usize,
    pub splits_skipped: usize,
}

struct LatestDiscoveryIdentity {
    source: String,
    source_device_id: String,
    display_name: Option<String>,
    identity_id: String,
}

#[derive(Debug, Clone)]
struct IntentSnapshot {
    tracked_state: String,
    label: Option<String>,
    alias: Option<String>,
    category: Option<String>,
    ssh_username: Option<String>,
    ssh_port: Option<i64>,
    endpoint_preference: Option<String>,
}

pub struct SqliteStore {
    conn: Connection,
    path: PathBuf,
}

impl SqliteStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating database directory {}", parent.display()))?;
        }

        let conn = Connection::open(&path)
            .with_context(|| format!("opening SQLite database {}", path.display()))?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "busy_timeout", 5000_i64)?;

        Ok(Self { conn, path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn migrate(&self) -> Result<()> {
        self.conn
            .execute_batch(INITIAL_MIGRATION)
            .context("applying initial SQLite migration")?;
        self.ensure_post_initial_columns()?;
        Ok(())
    }

    fn ensure_post_initial_columns(&self) -> Result<()> {
        if !self.table_has_column("device_identities", "merged_into_identity_id")? {
            self.conn.execute(
                "ALTER TABLE device_identities ADD COLUMN merged_into_identity_id TEXT REFERENCES device_identities(id) ON DELETE SET NULL",
                [],
            )?;
        }
        if !self.table_has_column("discovered_devices", "identity_override_id")? {
            self.conn.execute(
                "ALTER TABLE discovered_devices ADD COLUMN identity_override_id TEXT REFERENCES device_identities(id) ON DELETE SET NULL",
                [],
            )?;
        }
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_device_identities_merged ON device_identities(merged_into_identity_id)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_discovered_devices_identity_override ON discovered_devices(identity_override_id)",
            [],
        )?;
        Ok(())
    }

    fn table_has_column(&self, table: &str, column: &str) -> Result<bool> {
        let mut stmt = self.conn.prepare(&format!("PRAGMA table_info({table})"))?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
        for row in rows {
            if row? == column {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn set_daemon_started(&self) -> Result<()> {
        let now = now_timestamp();
        self.set_metadata("daemon_state", "online")?;
        self.set_metadata("daemon_started_at", &now)?;
        self.set_metadata("daemon_updated_at", &now)?;
        Ok(())
    }

    pub fn set_daemon_heartbeat(&self) -> Result<()> {
        self.set_metadata("daemon_state", "online")?;
        self.set_metadata("daemon_updated_at", &now_timestamp())?;
        Ok(())
    }

    pub fn set_metadata(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO daemon_metadata(key, value, updated_at)
             VALUES (?1, ?2, CURRENT_TIMESTAMP)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn daemon_status(&self, source: impl Into<String>) -> Result<DaemonStatus> {
        let state = self
            .metadata_value("daemon_state")?
            .unwrap_or_else(|| "unknown".to_string());
        let started_at = self.metadata_value("daemon_started_at")?;
        let updated_at = self.metadata_value("daemon_updated_at")?;

        Ok(DaemonStatus {
            state,
            source: source.into(),
            started_at,
            updated_at,
            db_path: Some(self.path.display().to_string()),
            stale: false,
        })
    }

    pub fn list_device_identities(&self) -> Result<Vec<DeviceIdentityRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT
                i.id,
                i.stable_key,
                ui.label,
                ui.alias,
                COALESCE(ui.tracked_state, 'untracked') AS tracked_state,
                ui.category,
                ui.ssh_username,
                ui.ssh_port,
                COALESCE(ui.endpoint_preference, 'auto') AS endpoint_preference,
                MAX(e.last_seen_at) AS last_seen_at,
                COUNT(e.id) AS endpoint_count
             FROM device_identities i
             LEFT JOIN device_user_intent ui ON ui.identity_id = i.id
             LEFT JOIN network_endpoints e ON e.identity_id = i.id
             WHERE i.merged_into_identity_id IS NULL
             GROUP BY i.id, i.stable_key, ui.label, ui.alias, ui.tracked_state, ui.category, ui.ssh_username, ui.ssh_port, ui.endpoint_preference
             ORDER BY COALESCE(ui.alias, ui.label, i.stable_key)",
        )?;

        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let tags = self
                .tags_for_identity(&id)
                .map_err(rusqlite::Error::ToSqlConversionFailure)?;
            let tracked_state_text: String = row.get(4)?;
            let endpoint_preference_text: String = row.get(8)?;
            let ssh_port: Option<i64> = row.get(7)?;
            let endpoint_count: i64 = row.get(10)?;
            Ok(DeviceIdentityRecord {
                identity: DeviceIdentity {
                    id,
                    stable_key: row.get(1)?,
                    label: row.get(2)?,
                    alias: row.get(3)?,
                    tracked_state: parse_row_enum(4, &tracked_state_text)?,
                    category: row.get(5)?,
                    tags,
                    ssh_username: row.get(6)?,
                    ssh_port: parse_optional_port(7, ssh_port)?,
                    endpoint_preference: parse_row_enum(8, &endpoint_preference_text)?,
                    last_seen_at: row.get(9)?,
                },
                endpoint_count: parse_nonnegative_count(10, endpoint_count)?,
            })
        })?;

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .context("listing device identities")
    }

    pub fn list_discovered_devices(&self) -> Result<Vec<DiscoveredDeviceRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT
                d.id,
                d.source,
                d.source_device_id,
                d.display_name,
                d.first_seen_at,
                d.last_seen_at,
                COALESCE(
                    d.identity_override_id,
                    (
                        SELECT o.identity_id
                        FROM discovery_observations o
                        WHERE o.discovered_device_id = d.id AND o.identity_id IS NOT NULL
                        ORDER BY o.observed_at DESC, o.id DESC
                        LIMIT 1
                    )
                ) AS identity_id
             FROM discovered_devices d
             ORDER BY d.last_seen_at DESC, d.display_name, d.source_device_id",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(DiscoveredDeviceRecord {
                device: DiscoveredDevice {
                    id: row.get(0)?,
                    source: row.get(1)?,
                    source_device_id: row.get(2)?,
                    display_name: row.get(3)?,
                    first_seen_at: row.get(4)?,
                    last_seen_at: row.get(5)?,
                },
                identity_id: row.get(6)?,
            })
        })?;

        let mut records = rows
            .collect::<std::result::Result<Vec<_>, _>>()
            .context("listing discovered devices")?;
        for record in &mut records {
            if let Some(identity_id) = record.identity_id.take() {
                record.identity_id = self.active_identity_id(&identity_id)?;
            }
        }
        Ok(records)
    }

    pub fn endpoints_for_identity(&self, identity_id: &str) -> Result<Vec<NetworkEndpoint>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, identity_id, kind, address, port, hostname, reachable_state, ssh_capability_state, last_seen_at, last_checked_at
             FROM network_endpoints
             WHERE identity_id = ?1
             ORDER BY kind, address",
        )?;

        let rows = stmt.query_map(params![identity_id], endpoint_from_row)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .context("listing network endpoints")
    }

    pub fn list_endpoints_for_probe(&self, tracked_only: bool) -> Result<Vec<NetworkEndpoint>> {
        let sql = if tracked_only {
            "SELECT e.id, e.identity_id, e.kind, e.address, e.port, e.hostname, e.reachable_state, e.ssh_capability_state, e.last_seen_at, e.last_checked_at
             FROM network_endpoints e
             JOIN device_user_intent ui ON ui.identity_id = e.identity_id
             WHERE ui.tracked_state = 'tracked'
             ORDER BY e.kind, e.address"
        } else {
            "SELECT e.id, e.identity_id, e.kind, e.address, e.port, e.hostname, e.reachable_state, e.ssh_capability_state, e.last_seen_at, e.last_checked_at
             FROM network_endpoints e
             ORDER BY e.kind, e.address"
        };

        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map([], endpoint_from_row)?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .context("listing endpoints for probing")
    }

    pub fn mark_lan_ips_reachable(&self, ips: &[String]) -> Result<usize> {
        let mut changed = 0;
        for ip in ips {
            changed += self.conn.execute(
                "UPDATE network_endpoints
                 SET reachable_state = 'online',
                     last_seen_at = CURRENT_TIMESTAMP,
                     last_checked_at = CURRENT_TIMESTAMP,
                     updated_at = CURRENT_TIMESTAMP
                 WHERE kind = 'lan_ip' AND address = ?1",
                params![ip],
            )?;
        }
        Ok(changed)
    }

    pub fn set_endpoint_probe_result(
        &self,
        endpoint_id: &str,
        reachability: Option<AvailabilityState>,
        ssh_capability: AvailabilityState,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE network_endpoints
             SET reachable_state = COALESCE(?2, reachable_state),
                 ssh_capability_state = ?3,
                 last_seen_at = CASE WHEN ?2 = 'online' THEN CURRENT_TIMESTAMP ELSE last_seen_at END,
                 last_checked_at = CURRENT_TIMESTAMP,
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = ?1",
            params![
                endpoint_id,
                reachability.map(|state| state.as_str().to_string()),
                ssh_capability.as_str()
            ],
        )?;
        Ok(())
    }

    pub fn mark_stale_endpoint_checks_unknown(&self, older_than_seconds: i64) -> Result<usize> {
        let changed = self.conn.execute(
            "UPDATE network_endpoints
             SET reachable_state = 'unknown',
                 ssh_capability_state = 'unknown',
                 updated_at = CURRENT_TIMESTAMP
             WHERE last_checked_at IS NULL
                OR last_checked_at < datetime('now', ?1)",
            params![format!("-{older_than_seconds} seconds")],
        )?;
        Ok(changed)
    }

    pub fn find_identity_id(&self, query: &str) -> Result<IdentityLookup> {
        let mut stmt = self.conn.prepare(
            "SELECT id FROM (
                SELECT i.id AS id, 0 AS priority
                FROM device_identities i
                LEFT JOIN device_user_intent ui ON ui.identity_id = i.id
                WHERE ui.alias = ?1

                UNION ALL
                SELECT i.id AS id, 1 AS priority
                FROM device_identities i
                LEFT JOIN device_user_intent ui ON ui.identity_id = i.id
                WHERE ui.label = ?1

                UNION ALL
                SELECT i.id AS id, 2 AS priority
                FROM device_identities i
                WHERE i.id = ?1 OR i.stable_key = ?1

                UNION ALL
                SELECT COALESCE(d.identity_override_id, o.identity_id) AS id, 3 AS priority
                FROM discovered_devices d
                JOIN discovery_observations o ON o.discovered_device_id = d.id
                WHERE o.identity_id IS NOT NULL
                  AND (d.id = ?1 OR d.source_device_id = ?1 OR d.display_name = ?1)

                UNION ALL
                SELECT e.identity_id AS id, 4 AS priority
                FROM network_endpoints e
                WHERE e.address = ?1 OR e.hostname = ?1
             )
             WHERE id IS NOT NULL
             GROUP BY id
             ORDER BY MIN(priority), id",
        )?;
        let raw_matches = stmt
            .query_map(params![query], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        let mut matches = Vec::new();
        for raw_id in raw_matches {
            if let Some(active_id) = self.active_identity_id(&raw_id)? {
                if !matches.contains(&active_id) {
                    matches.push(active_id);
                }
            }
        }

        Ok(match matches.len() {
            0 => IdentityLookup::NotFound,
            1 => IdentityLookup::Found(matches[0].clone()),
            _ => IdentityLookup::Ambiguous(matches),
        })
    }

    pub fn record_tailscale_nodes(
        &self,
        tailnet: Option<&str>,
        nodes: &[TailscaleNodeObservation],
    ) -> Result<usize> {
        if let Some(tailnet) = tailnet.filter(|tailnet| !tailnet.is_empty()) {
            self.set_metadata("tailscale_tailnet", tailnet)?;
        }

        for node in nodes {
            let stable_key = format!("tailscale:{}", node.source_device_id);
            let discovered_id =
                self.discovered_id_for_source("tailscale", &node.source_device_id)?;
            let identity_id = self
                .identity_id_for_discovery("tailscale", &node.source_device_id, Some(&stable_key))?
                .context("Tailscale observations require an identity")?;
            let display_name = node
                .display_name
                .clone()
                .or_else(|| node.dns_name.clone())
                .unwrap_or_else(|| node.source_device_id.clone());
            let state = match node.online {
                Some(true) => "online",
                Some(false) => "offline",
                None => "unknown",
            };

            self.conn.execute(
                "INSERT INTO discovered_devices(id, source, source_device_id, display_name, raw_json)
                 VALUES (?1, 'tailscale', ?2, ?3, ?4)
                 ON CONFLICT(source, source_device_id) DO UPDATE SET
                    display_name = excluded.display_name,
                    raw_json = excluded.raw_json,
                    last_seen_at = CURRENT_TIMESTAMP",
                params![discovered_id, node.source_device_id, display_name, node.raw_json],
            )?;

            self.conn.execute(
                "INSERT INTO discovery_observations(discovered_device_id, identity_id, source, evidence_json)
                 VALUES (?1, ?2, 'tailscale', ?3)",
                params![discovered_id, identity_id, node.raw_json],
            )?;

            self.conn.execute(
                "INSERT INTO identity_evidence(identity_id, discovered_device_id, evidence_type, evidence_value, confidence, source)
                 VALUES (?1, ?2, 'tailscale_node', ?3, 1.0, 'tailscale')",
                params![identity_id, discovered_id, node.source_device_id],
            )?;

            if let Some(hostname) = node.dns_name.as_deref().filter(|name| !name.is_empty()) {
                self.upsert_endpoint(
                    &identity_id,
                    "tailscale_dns",
                    hostname,
                    None,
                    Some(hostname),
                    "tailscale",
                    state,
                )?;
            }

            for ip in node.tailscale_ips.iter().filter(|ip| !ip.is_empty()) {
                self.upsert_endpoint(
                    &identity_id,
                    "tailscale_ip",
                    ip,
                    None,
                    None,
                    "tailscale",
                    state,
                )?;
            }
        }

        Ok(nodes.len())
    }

    pub fn record_lan_devices(&self, observations: &[LanDeviceObservation]) -> Result<usize> {
        for observation in observations {
            let source_device_id = observation
                .mac_address
                .as_deref()
                .map(normalize_mac)
                .filter(|value| !value.is_empty())
                .or_else(|| observation.hostname.clone())
                .unwrap_or_else(|| observation.ip_address.clone());
            let discovered_id = self.discovered_id_for_source("arp", &source_device_id)?;
            let display_name = observation
                .hostname
                .clone()
                .unwrap_or_else(|| observation.ip_address.clone());
            let raw_json = serde_json::to_string(observation)?;
            let fallback_stable_key = if let Some(mac) = observation
                .mac_address
                .as_deref()
                .map(normalize_mac)
                .filter(|value| !value.is_empty())
            {
                Some(format!("mac:{mac}"))
            } else {
                observation
                    .hostname
                    .as_deref()
                    .filter(|value| !value.is_empty())
                    .map(|hostname| format!("lan-host:{hostname}"))
            };
            let identity_id = self.identity_id_for_discovery(
                "arp",
                &source_device_id,
                fallback_stable_key.as_deref(),
            )?;

            self.conn.execute(
                "INSERT INTO discovered_devices(id, source, source_device_id, display_name, raw_json)
                 VALUES (?1, 'arp', ?2, ?3, ?4)
                 ON CONFLICT(source, source_device_id) DO UPDATE SET
                    display_name = excluded.display_name,
                    raw_json = excluded.raw_json,
                    last_seen_at = CURRENT_TIMESTAMP",
                params![discovered_id, source_device_id, display_name, raw_json],
            )?;

            self.conn.execute(
                "INSERT INTO discovery_observations(discovered_device_id, identity_id, source, interface_name, evidence_json)
                 VALUES (?1, ?2, 'arp', ?3, ?4)",
                params![
                    discovered_id,
                    identity_id,
                    observation.interface_name,
                    raw_json
                ],
            )?;

            if let Some(identity_id) = identity_id {
                if let Some(mac) = observation
                    .mac_address
                    .as_deref()
                    .map(normalize_mac)
                    .filter(|value| !value.is_empty())
                {
                    self.conn.execute(
                        "INSERT INTO identity_evidence(identity_id, discovered_device_id, evidence_type, evidence_value, confidence, source)
                         VALUES (?1, ?2, 'mac_address', ?3, 0.85, 'arp')",
                        params![identity_id, discovered_id, mac],
                    )?;
                }

                if let Some(hostname) = observation
                    .hostname
                    .as_deref()
                    .filter(|value| !value.is_empty())
                {
                    self.conn.execute(
                        "INSERT INTO identity_evidence(identity_id, discovered_device_id, evidence_type, evidence_value, confidence, source)
                         VALUES (?1, ?2, 'hostname', ?3, 0.60, 'arp')",
                        params![identity_id, discovered_id, hostname],
                    )?;
                    self.upsert_endpoint(
                        &identity_id,
                        "lan_dns",
                        hostname,
                        None,
                        Some(hostname),
                        "arp",
                        "unknown",
                    )?;
                }

                self.upsert_endpoint(
                    &identity_id,
                    "lan_ip",
                    &observation.ip_address,
                    None,
                    None,
                    "arp",
                    "unknown",
                )?;
            }
        }

        Ok(observations.len())
    }

    pub fn record_mdns_services(&self, observations: &[MdnsServiceObservation]) -> Result<usize> {
        for observation in observations {
            let source_device_id = if observation.source_device_id.trim().is_empty() {
                format!(
                    "{}:{}:{}",
                    observation.domain, observation.service_type, observation.service_name
                )
            } else {
                observation.source_device_id.clone()
            };
            let discovered_id = self.discovered_id_for_source("mdns", &source_device_id)?;
            let raw_json = serde_json::to_string(observation)?;
            let fallback_stable_key = observation
                .hostname
                .as_deref()
                .filter(|value| !value.is_empty())
                .map(|hostname| format!("mdns-host:{}", normalize_hostname(hostname)))
                .unwrap_or_else(|| {
                    format!(
                        "mdns-service:{}:{}:{}",
                        observation.domain, observation.service_type, observation.service_name
                    )
                });
            let identity_id = self
                .identity_id_for_discovery("mdns", &source_device_id, Some(&fallback_stable_key))?
                .context("mDNS observations require an identity")?;

            self.conn.execute(
                "INSERT INTO discovered_devices(id, source, source_device_id, display_name, raw_json)
                 VALUES (?1, 'mdns', ?2, ?3, ?4)
                 ON CONFLICT(source, source_device_id) DO UPDATE SET
                    display_name = excluded.display_name,
                    raw_json = excluded.raw_json,
                    last_seen_at = CURRENT_TIMESTAMP",
                params![
                    discovered_id,
                    source_device_id,
                    observation.service_name,
                    raw_json
                ],
            )?;

            self.conn.execute(
                "INSERT INTO discovery_observations(discovered_device_id, identity_id, source, evidence_json)
                 VALUES (?1, ?2, 'mdns', ?3)",
                params![discovered_id, identity_id, raw_json],
            )?;

            self.conn.execute(
                "INSERT INTO identity_evidence(identity_id, discovered_device_id, evidence_type, evidence_value, confidence, source)
                 VALUES (?1, ?2, 'mdns_service', ?3, 0.55, 'mdns')",
                params![identity_id, discovered_id, observation.service_name],
            )?;

            if let Some(hostname) = observation
                .hostname
                .as_deref()
                .filter(|value| !value.is_empty())
            {
                let hostname = normalize_hostname(hostname);
                self.conn.execute(
                    "INSERT INTO identity_evidence(identity_id, discovered_device_id, evidence_type, evidence_value, confidence, source)
                     VALUES (?1, ?2, 'hostname', ?3, 0.65, 'mdns')",
                    params![identity_id, discovered_id, hostname.as_str()],
                )?;
                self.upsert_endpoint(
                    &identity_id,
                    "mdns",
                    &hostname,
                    if observation.service_type == "_ssh._tcp" {
                        observation.port
                    } else {
                        None
                    },
                    Some(&hostname),
                    "mdns",
                    "online",
                )?;

                for ip_address in observation
                    .ip_addresses
                    .iter()
                    .map(|value| value.trim())
                    .filter(|value| !value.is_empty())
                {
                    self.conn.execute(
                        "INSERT INTO identity_evidence(identity_id, discovered_device_id, evidence_type, evidence_value, confidence, source)
                         VALUES (?1, ?2, 'ip_address', ?3, 0.65, 'mdns')",
                        params![identity_id, discovered_id, ip_address],
                    )?;
                    self.upsert_endpoint(
                        &identity_id,
                        "lan_ip",
                        ip_address,
                        if observation.service_type == "_ssh._tcp" {
                            observation.port
                        } else {
                            None
                        },
                        None,
                        "mdns",
                        "online",
                    )?;
                }
            }
        }

        Ok(observations.len())
    }

    pub fn record_resolved_endpoint_ips(
        &self,
        endpoint: &NetworkEndpoint,
        ip_addresses: &[String],
    ) -> Result<usize> {
        if !matches!(endpoint.kind, EndpointKind::LanDns | EndpointKind::Mdns) {
            return Ok(0);
        }

        let mut changed = 0;
        for ip_address in ip_addresses
            .iter()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
        {
            self.upsert_endpoint(
                &endpoint.identity_id,
                "lan_ip",
                ip_address,
                endpoint.port,
                None,
                "mdns",
                "online",
            )?;
            changed += 1;
        }
        Ok(changed)
    }

    pub fn device_identity_record(
        &self,
        identity_id: &str,
    ) -> Result<Option<DeviceIdentityRecord>> {
        Ok(self
            .list_device_identities()?
            .into_iter()
            .find(|record| record.identity.id == identity_id))
    }

    pub fn device_details_by_id(&self, identity_id: &str) -> Result<Option<DeviceDetails>> {
        let Some(record) = self.device_identity_record(identity_id)? else {
            return Ok(None);
        };
        Ok(Some(DeviceDetails {
            identity: record.identity,
            endpoints: self.endpoints_for_identity(identity_id)?,
        }))
    }

    pub fn set_tracked_state_by_id(
        &self,
        identity_id: &str,
        state: TrackedState,
        label: Option<&str>,
        alias: Option<&str>,
    ) -> Result<DeviceMutationResult> {
        self.ensure_intent_row(identity_id)?;
        if let Some(label) = label {
            self.conn.execute(
                "UPDATE device_user_intent SET label = ?2, updated_at = CURRENT_TIMESTAMP WHERE identity_id = ?1",
                params![identity_id, label],
            )?;
        }

        let alias = if state == TrackedState::Tracked {
            let current_alias = self.current_alias(identity_id)?;
            Some(match alias {
                Some(alias) => self.unique_alias(identity_id, &slug_alias(alias))?,
                None => match current_alias {
                    Some(alias) => alias,
                    None => {
                        let seed = self.alias_seed_for_identity(identity_id)?;
                        self.unique_alias(identity_id, &slug_alias(&seed))?
                    }
                },
            })
        } else {
            alias
                .map(|alias| self.unique_alias(identity_id, &slug_alias(alias)))
                .transpose()?
        };

        if let Some(alias) = alias {
            self.conn.execute(
                "UPDATE device_user_intent SET tracked_state = ?2, alias = ?3, updated_at = CURRENT_TIMESTAMP WHERE identity_id = ?1",
                params![identity_id, state.as_str(), alias],
            )?;
        } else {
            self.conn.execute(
                "UPDATE device_user_intent SET tracked_state = ?2, updated_at = CURRENT_TIMESTAMP WHERE identity_id = ?1",
                params![identity_id, state.as_str()],
            )?;
        }

        let record = self
            .device_identity_record(identity_id)?
            .context("updated identity disappeared")?;
        Ok(DeviceMutationResult {
            identity: record.identity,
            endpoint_count: record.endpoint_count,
            message: format!("device is now {}", state.as_str()),
        })
    }

    pub fn set_label_by_id(&self, identity_id: &str, label: &str) -> Result<DeviceMutationResult> {
        self.ensure_intent_row(identity_id)?;
        self.conn.execute(
            "UPDATE device_user_intent SET label = ?2, updated_at = CURRENT_TIMESTAMP WHERE identity_id = ?1",
            params![identity_id, label],
        )?;
        self.mutation_result(identity_id, "label updated")
    }

    pub fn set_alias_by_id(&self, identity_id: &str, alias: &str) -> Result<DeviceMutationResult> {
        self.ensure_intent_row(identity_id)?;
        let alias = self.unique_alias(identity_id, &slug_alias(alias))?;
        self.conn.execute(
            "UPDATE device_user_intent SET alias = ?2, updated_at = CURRENT_TIMESTAMP WHERE identity_id = ?1",
            params![identity_id, alias],
        )?;
        self.mutation_result(identity_id, "alias updated")
    }

    pub fn set_category_by_id(
        &self,
        identity_id: &str,
        category: Option<&str>,
    ) -> Result<DeviceMutationResult> {
        self.ensure_intent_row(identity_id)?;
        self.conn.execute(
            "UPDATE device_user_intent SET category = ?2, updated_at = CURRENT_TIMESTAMP WHERE identity_id = ?1",
            params![identity_id, category],
        )?;
        self.mutation_result(identity_id, "category updated")
    }

    pub fn add_tag_by_id(&self, identity_id: &str, tag: &str) -> Result<DeviceMutationResult> {
        self.ensure_intent_row(identity_id)?;
        let tag = normalize_tag(tag)?;
        self.conn.execute(
            "INSERT OR IGNORE INTO device_tags(identity_id, tag) VALUES (?1, ?2)",
            params![identity_id, tag],
        )?;
        self.conn.execute(
            "UPDATE device_user_intent SET updated_at = CURRENT_TIMESTAMP WHERE identity_id = ?1",
            params![identity_id],
        )?;
        self.mutation_result(identity_id, "tag added")
    }

    pub fn remove_tag_by_id(&self, identity_id: &str, tag: &str) -> Result<DeviceMutationResult> {
        let tag = normalize_tag(tag)?;
        self.conn.execute(
            "DELETE FROM device_tags WHERE identity_id = ?1 AND tag = ?2",
            params![identity_id, tag],
        )?;
        self.conn.execute(
            "UPDATE device_user_intent SET updated_at = CURRENT_TIMESTAMP WHERE identity_id = ?1",
            params![identity_id],
        )?;
        self.mutation_result(identity_id, "tag removed")
    }

    pub fn set_ssh_username_by_id(
        &self,
        identity_id: &str,
        username: Option<&str>,
    ) -> Result<DeviceMutationResult> {
        self.ensure_intent_row(identity_id)?;
        self.conn.execute(
            "UPDATE device_user_intent SET ssh_username = ?2, updated_at = CURRENT_TIMESTAMP WHERE identity_id = ?1",
            params![identity_id, username],
        )?;
        self.mutation_result(identity_id, "SSH username updated")
    }

    pub fn set_ssh_port_by_id(
        &self,
        identity_id: &str,
        port: Option<u16>,
    ) -> Result<DeviceMutationResult> {
        self.ensure_intent_row(identity_id)?;
        self.conn.execute(
            "UPDATE device_user_intent SET ssh_port = ?2, updated_at = CURRENT_TIMESTAMP WHERE identity_id = ?1",
            params![identity_id, port.map(i64::from)],
        )?;
        self.mutation_result(identity_id, "SSH port updated")
    }

    pub fn set_endpoint_preference_by_id(
        &self,
        identity_id: &str,
        preference: EndpointPreference,
    ) -> Result<DeviceMutationResult> {
        self.ensure_intent_row(identity_id)?;
        self.conn.execute(
            "UPDATE device_user_intent SET endpoint_preference = ?2, updated_at = CURRENT_TIMESTAMP WHERE identity_id = ?1",
            params![identity_id, preference.as_str()],
        )?;
        self.mutation_result(identity_id, "endpoint preference updated")
    }

    pub fn merge_identities_by_id(
        &self,
        source_identity_id: &str,
        target_identity_id: &str,
        reason: Option<&str>,
    ) -> Result<IdentityCorrectionResult> {
        if source_identity_id == target_identity_id {
            bail!("cannot merge an identity into itself");
        }
        self.assert_active_identity(source_identity_id)?;
        self.assert_active_identity(target_identity_id)?;

        let tx = self.conn.unchecked_transaction()?;
        let source_family = merged_family_ids(&tx, source_identity_id)?;
        for merged_source_id in &source_family {
            merge_endpoints(&tx, merged_source_id, target_identity_id)?;
            tx.execute(
                "UPDATE discovery_observations SET identity_id = ?2 WHERE identity_id = ?1",
                params![merged_source_id, target_identity_id],
            )?;
            tx.execute(
                "UPDATE identity_evidence SET identity_id = ?2 WHERE identity_id = ?1",
                params![merged_source_id, target_identity_id],
            )?;
            tx.execute(
                "INSERT OR IGNORE INTO device_tags(identity_id, tag) SELECT ?2, tag FROM device_tags WHERE identity_id = ?1",
                params![merged_source_id, target_identity_id],
            )?;
            tx.execute(
                "DELETE FROM device_tags WHERE identity_id = ?1",
                params![merged_source_id],
            )?;
            tx.execute(
                "UPDATE discovered_devices SET identity_override_id = ?2 WHERE identity_override_id = ?1",
                params![merged_source_id, target_identity_id],
            )?;
        }
        merge_user_intent(&tx, source_identity_id, target_identity_id)?;
        tx.execute(
            "INSERT INTO identity_corrections(correction_type, from_identity_id, to_identity_id, reason) VALUES ('merge', ?1, ?2, ?3)",
            params![source_identity_id, target_identity_id, reason],
        )?;
        tx.execute(
            "UPDATE device_identities
             SET merged_into_identity_id = ?2, updated_at = CURRENT_TIMESTAMP
             WHERE id = ?1 OR merged_into_identity_id = ?1",
            params![source_identity_id, target_identity_id],
        )?;
        tx.commit()?;

        Ok(IdentityCorrectionResult {
            identity_id: target_identity_id.to_string(),
            affected_identity_id: source_identity_id.to_string(),
            message: format!("merged {source_identity_id} into {target_identity_id}"),
        })
    }

    pub fn split_discovered_device_by_id(
        &self,
        discovered_device_id: &str,
        reason: Option<&str>,
    ) -> Result<IdentityCorrectionResult> {
        if let Some(existing_identity_id) =
            self.identity_override_for_discovered_id(discovered_device_id)?
        {
            bail!(
                "discovered device '{discovered_device_id}' is already split to identity '{existing_identity_id}'"
            );
        }
        let latest = self
            .latest_discovery_identity(discovered_device_id)?
            .with_context(|| {
                format!(
                    "discovered device '{discovered_device_id}' was not found or has no identity"
                )
            })?;
        let old_identity_id = self.resolve_active_identity_id(&latest.identity_id)?;
        let new_identity_id = format!("identity-{}", Uuid::new_v4());
        let stable_key = format!("split:{}:{}", latest.source, latest.source_device_id);

        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO device_identities(id, stable_key) VALUES (?1, ?2)",
            params![new_identity_id, stable_key],
        )?;
        tx.execute(
            "UPDATE discovered_devices SET identity_override_id = ?2 WHERE id = ?1",
            params![discovered_device_id, new_identity_id],
        )?;
        tx.execute(
            "UPDATE discovery_observations SET identity_id = ?2 WHERE discovered_device_id = ?1",
            params![discovered_device_id, new_identity_id],
        )?;
        tx.execute(
            "UPDATE identity_evidence SET identity_id = ?2 WHERE discovered_device_id = ?1",
            params![discovered_device_id, new_identity_id],
        )?;
        tx.execute(
            "INSERT INTO identity_corrections(correction_type, from_identity_id, to_identity_id, reason) VALUES ('split', ?1, ?2, ?3)",
            params![old_identity_id, new_identity_id, reason],
        )?;
        move_split_endpoints(
            &tx,
            &old_identity_id,
            &new_identity_id,
            discovered_device_id,
        )?;
        if let Some(label) = latest
            .display_name
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            tx.execute(
                "INSERT INTO device_user_intent(identity_id, tracked_state, label) VALUES (?1, 'untracked', ?2)",
                params![new_identity_id, label],
            )?;
        }
        tx.commit()?;

        Ok(IdentityCorrectionResult {
            identity_id: new_identity_id,
            affected_identity_id: old_identity_id,
            message: format!("split discovered device {discovered_device_id} into a new identity"),
        })
    }

    pub fn export_user_settings(&self) -> Result<UserSettingsExport> {
        let devices = self
            .list_device_identities()?
            .into_iter()
            .map(|record| record.identity)
            .filter(should_export_device_settings)
            .map(|identity| DeviceSettingsExport {
                stable_key: identity.stable_key,
                tracked_state: identity.tracked_state,
                label: identity.label,
                alias: identity.alias,
                category: identity.category,
                tags: identity.tags,
                ssh_username: identity.ssh_username,
                ssh_port: identity.ssh_port,
                endpoint_preference: identity.endpoint_preference,
            })
            .collect();

        Ok(UserSettingsExport {
            format_version: 1,
            devices,
            merges: self.export_merge_settings()?,
            splits: self.export_split_settings()?,
        })
    }

    pub fn import_user_settings(
        &self,
        export: &UserSettingsExport,
        dry_run: bool,
    ) -> Result<UserSettingsImportResult> {
        if export.format_version != 1 {
            bail!(
                "unsupported user settings export format version {}",
                export.format_version
            );
        }

        let mut result = UserSettingsImportResult {
            dry_run,
            devices_applied: 0,
            devices_missing: 0,
            merges_applied: 0,
            merges_skipped: 0,
            splits_applied: 0,
            splits_skipped: 0,
        };

        for split in &export.splits {
            let Some(discovered_id) =
                self.discovered_id_for_existing_source(&split.source, &split.source_device_id)?
            else {
                result.splits_skipped += 1;
                continue;
            };
            if self
                .identity_override_for_source(&split.source, &split.source_device_id)?
                .is_some()
            {
                result.splits_skipped += 1;
                continue;
            }
            if !dry_run {
                self.split_discovered_device_by_id(&discovered_id, split.reason.as_deref())?;
            }
            result.splits_applied += 1;
        }

        for merge in &export.merges {
            let Some(source_id) =
                self.identity_id_for_existing_stable_key(&merge.source_stable_key)?
            else {
                result.merges_skipped += 1;
                continue;
            };
            let Some(target_id) =
                self.identity_id_for_existing_stable_key(&merge.target_stable_key)?
            else {
                result.merges_skipped += 1;
                continue;
            };
            if source_id == target_id {
                result.merges_skipped += 1;
                continue;
            }
            if !dry_run {
                self.merge_identities_by_id(&source_id, &target_id, merge.reason.as_deref())?;
            }
            result.merges_applied += 1;
        }

        for device in &export.devices {
            let Some(identity_id) = self.identity_id_for_existing_stable_key(&device.stable_key)?
            else {
                result.devices_missing += 1;
                continue;
            };
            if !dry_run {
                self.apply_device_settings(&identity_id, device)?;
            }
            result.devices_applied += 1;
        }

        Ok(result)
    }

    pub fn insert_test_identity(&self, stable_key: &str, alias: Option<&str>) -> Result<String> {
        let id = format!("identity-{}", Uuid::new_v4());
        self.conn.execute(
            "INSERT INTO device_identities(id, stable_key) VALUES (?1, ?2)",
            params![id, stable_key],
        )?;
        self.conn.execute(
            "INSERT INTO device_user_intent(identity_id, tracked_state, alias) VALUES (?1, 'tracked', ?2)",
            params![id, alias],
        )?;
        Ok(id)
    }

    fn export_merge_settings(&self) -> Result<Vec<MergeSettingsExport>> {
        let mut stmt = self.conn.prepare(
            "SELECT source.stable_key,
                    target.stable_key,
                    (
                        SELECT reason
                        FROM identity_corrections c
                        WHERE c.correction_type = 'merge' AND c.from_identity_id = source.id
                        ORDER BY c.created_at DESC, c.id DESC
                        LIMIT 1
                    ) AS reason
             FROM device_identities source
             JOIN device_identities target ON target.id = source.merged_into_identity_id
             ORDER BY source.stable_key",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(MergeSettingsExport {
                source_stable_key: row.get(0)?,
                target_stable_key: row.get(1)?,
                reason: row.get(2)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .context("exporting merge settings")
    }

    fn export_split_settings(&self) -> Result<Vec<SplitSettingsExport>> {
        let mut stmt = self.conn.prepare(
            "SELECT d.source,
                    d.source_device_id,
                    i.stable_key,
                    (
                        SELECT reason
                        FROM identity_corrections c
                        WHERE c.correction_type = 'split' AND c.to_identity_id = d.identity_override_id
                        ORDER BY c.created_at DESC, c.id DESC
                        LIMIT 1
                    ) AS reason
             FROM discovered_devices d
             JOIN device_identities i ON i.id = d.identity_override_id
             WHERE d.identity_override_id IS NOT NULL
             ORDER BY d.source, d.source_device_id",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(SplitSettingsExport {
                source: row.get(0)?,
                source_device_id: row.get(1)?,
                target_stable_key: row.get(2)?,
                reason: row.get(3)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()
            .context("exporting split settings")
    }

    fn identity_id_for_existing_stable_key(&self, stable_key: &str) -> Result<Option<String>> {
        let identity_id: Option<String> = self
            .conn
            .query_row(
                "SELECT id FROM device_identities WHERE stable_key = ?1",
                params![stable_key],
                |row| row.get(0),
            )
            .optional()
            .context("looking up identity by stable key")?;
        identity_id
            .map(|identity_id| self.active_identity_id(&identity_id))
            .transpose()
            .map(Option::flatten)
    }

    fn discovered_id_for_existing_source(
        &self,
        source: &str,
        source_device_id: &str,
    ) -> Result<Option<String>> {
        self.conn
            .query_row(
                "SELECT id FROM discovered_devices WHERE source = ?1 AND source_device_id = ?2",
                params![source, source_device_id],
                |row| row.get(0),
            )
            .optional()
            .context("looking up discovered device by source")
    }

    fn apply_device_settings(
        &self,
        identity_id: &str,
        settings: &DeviceSettingsExport,
    ) -> Result<()> {
        self.ensure_intent_row(identity_id)?;
        let alias = settings
            .alias
            .as_deref()
            .map(|alias| self.unique_alias(identity_id, &slug_alias(alias)))
            .transpose()?;
        self.conn.execute(
            "UPDATE device_user_intent
             SET tracked_state = ?2,
                 label = ?3,
                 alias = ?4,
                 category = ?5,
                 ssh_username = ?6,
                 ssh_port = ?7,
                 endpoint_preference = ?8,
                 updated_at = CURRENT_TIMESTAMP
             WHERE identity_id = ?1",
            params![
                identity_id,
                settings.tracked_state.as_str(),
                settings.label.as_deref(),
                alias.as_deref(),
                settings.category.as_deref(),
                settings.ssh_username.as_deref(),
                settings.ssh_port.map(i64::from),
                settings.endpoint_preference.as_str(),
            ],
        )?;
        self.conn.execute(
            "DELETE FROM device_tags WHERE identity_id = ?1",
            params![identity_id],
        )?;
        for tag in &settings.tags {
            let tag = normalize_tag(tag)?;
            self.conn.execute(
                "INSERT OR IGNORE INTO device_tags(identity_id, tag) VALUES (?1, ?2)",
                params![identity_id, tag],
            )?;
        }
        Ok(())
    }

    fn assert_active_identity(&self, identity_id: &str) -> Result<()> {
        let exists: Option<String> = self
            .conn
            .query_row(
                "SELECT id FROM device_identities WHERE id = ?1 AND merged_into_identity_id IS NULL",
                params![identity_id],
                |row| row.get(0),
            )
            .optional()?;
        if exists.is_none() {
            bail!("active identity '{identity_id}' was not found");
        }
        Ok(())
    }

    fn latest_discovery_identity(
        &self,
        discovered_device_id: &str,
    ) -> Result<Option<LatestDiscoveryIdentity>> {
        self.conn
            .query_row(
                "SELECT d.source, d.source_device_id, d.display_name, o.identity_id
                 FROM discovered_devices d
                 JOIN discovery_observations o ON o.discovered_device_id = d.id
                 WHERE d.id = ?1 AND o.identity_id IS NOT NULL
                 ORDER BY o.observed_at DESC, o.id DESC
                 LIMIT 1",
                params![discovered_device_id],
                |row| {
                    Ok(LatestDiscoveryIdentity {
                        source: row.get(0)?,
                        source_device_id: row.get(1)?,
                        display_name: row.get(2)?,
                        identity_id: row.get(3)?,
                    })
                },
            )
            .optional()
            .context("reading latest discovery identity")
    }

    fn identity_id_for_discovery(
        &self,
        source: &str,
        source_device_id: &str,
        fallback_stable_key: Option<&str>,
    ) -> Result<Option<String>> {
        if let Some(override_id) = self.identity_override_for_source(source, source_device_id)? {
            return Ok(Some(override_id));
        }
        if let Some(existing_id) = self.latest_identity_for_source(source, source_device_id)? {
            return Ok(Some(existing_id));
        }
        fallback_stable_key
            .map(|stable_key| self.identity_id_for_stable_key(stable_key))
            .transpose()
    }

    fn latest_identity_for_source(
        &self,
        source: &str,
        source_device_id: &str,
    ) -> Result<Option<String>> {
        let identity_id = self
            .conn
            .query_row(
                "SELECT o.identity_id
                 FROM discovered_devices d
                 JOIN discovery_observations o ON o.discovered_device_id = d.id
                 WHERE d.source = ?1 AND d.source_device_id = ?2 AND o.identity_id IS NOT NULL
                 ORDER BY o.observed_at DESC, o.id DESC
                 LIMIT 1",
                params![source, source_device_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        identity_id
            .map(|identity_id| self.resolve_active_identity_id(&identity_id))
            .transpose()
    }

    fn identity_override_for_source(
        &self,
        source: &str,
        source_device_id: &str,
    ) -> Result<Option<String>> {
        let override_id = self
            .conn
            .query_row(
                "SELECT identity_override_id FROM discovered_devices WHERE source = ?1 AND source_device_id = ?2",
                params![source, source_device_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten();
        override_id
            .map(|identity_id| self.resolve_active_identity_id(&identity_id))
            .transpose()
    }

    fn identity_override_for_discovered_id(
        &self,
        discovered_device_id: &str,
    ) -> Result<Option<String>> {
        let override_id = self
            .conn
            .query_row(
                "SELECT identity_override_id FROM discovered_devices WHERE id = ?1",
                params![discovered_device_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten();
        override_id
            .map(|identity_id| self.resolve_active_identity_id(&identity_id))
            .transpose()
    }

    fn identity_id_for_stable_key(&self, stable_key: &str) -> Result<String> {
        if let Some(id) = self
            .conn
            .query_row(
                "SELECT id FROM device_identities WHERE stable_key = ?1",
                params![stable_key],
                |row| row.get::<_, String>(0),
            )
            .optional()?
        {
            return self.resolve_active_identity_id(&id);
        }

        let id = format!("identity-{}", Uuid::new_v4());
        self.conn.execute(
            "INSERT INTO device_identities(id, stable_key) VALUES (?1, ?2)",
            params![id, stable_key],
        )?;
        Ok(id)
    }

    fn active_identity_id(&self, identity_id: &str) -> Result<Option<String>> {
        let mut current = identity_id.to_string();
        let mut seen = Vec::new();
        for _ in 0..32 {
            if seen.contains(&current) {
                bail!("identity merge cycle involving '{identity_id}'");
            }
            seen.push(current.clone());
            let row = self
                .conn
                .query_row(
                    "SELECT id, merged_into_identity_id FROM device_identities WHERE id = ?1",
                    params![current],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
                )
                .optional()?;
            let Some((id, merged_into_identity_id)) = row else {
                return Ok(None);
            };
            let Some(next_id) = merged_into_identity_id else {
                return Ok(Some(id));
            };
            current = next_id;
        }
        bail!("identity merge chain too deep for '{identity_id}'")
    }

    fn resolve_active_identity_id(&self, identity_id: &str) -> Result<String> {
        self.active_identity_id(identity_id)?
            .with_context(|| format!("identity '{identity_id}' was not found"))
    }

    fn discovered_id_for_source(&self, source: &str, source_device_id: &str) -> Result<String> {
        if let Some(id) = self
            .conn
            .query_row(
                "SELECT id FROM discovered_devices WHERE source = ?1 AND source_device_id = ?2",
                params![source, source_device_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?
        {
            return Ok(id);
        }

        Ok(format!("discovered-{}", Uuid::new_v4()))
    }

    #[allow(clippy::too_many_arguments)]
    fn upsert_endpoint(
        &self,
        identity_id: &str,
        kind: &str,
        address: &str,
        port: Option<u16>,
        hostname: Option<&str>,
        source: &str,
        reachability: &str,
    ) -> Result<()> {
        let existing_id = self
            .conn
            .query_row(
                "SELECT id FROM network_endpoints
                 WHERE identity_id = ?1 AND kind = ?2 AND address = ?3 AND COALESCE(port, 0) = COALESCE(?4, 0)",
                params![identity_id, kind, address, port.map(i64::from)],
                |row| row.get::<_, String>(0),
            )
            .optional()?;

        if let Some(id) = existing_id {
            self.conn.execute(
                "UPDATE network_endpoints SET
                    hostname = ?2,
                    source = ?3,
                    reachable_state = ?4,
                    last_seen_at = CASE WHEN ?4 = 'online' OR ?3 = 'arp' THEN CURRENT_TIMESTAMP ELSE last_seen_at END,
                    last_checked_at = CURRENT_TIMESTAMP,
                    updated_at = CURRENT_TIMESTAMP
                 WHERE id = ?1",
                params![id, hostname, source, reachability],
            )?;
        } else {
            self.conn.execute(
                "INSERT INTO network_endpoints(
                    id, identity_id, kind, address, port, hostname, source,
                    reachable_state, ssh_capability_state, last_seen_at, last_checked_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'unknown',
                    CASE WHEN ?8 = 'online' OR ?7 = 'arp' THEN CURRENT_TIMESTAMP ELSE NULL END,
                    CURRENT_TIMESTAMP)",
                params![
                    format!("endpoint-{}", Uuid::new_v4()),
                    identity_id,
                    kind,
                    address,
                    port.map(i64::from),
                    hostname,
                    source,
                    reachability
                ],
            )?;
        }
        Ok(())
    }

    fn ensure_intent_row(&self, identity_id: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO device_user_intent(identity_id, tracked_state)
             VALUES (?1, 'untracked')
             ON CONFLICT(identity_id) DO NOTHING",
            params![identity_id],
        )?;
        Ok(())
    }

    fn current_alias(&self, identity_id: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row(
                "SELECT alias FROM device_user_intent WHERE identity_id = ?1",
                params![identity_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()
            .context("reading current device alias")?
            .flatten())
    }

    fn alias_seed_for_identity(&self, identity_id: &str) -> Result<String> {
        self.conn
            .query_row(
                "SELECT COALESCE(
                    ui.label,
                    (
                        SELECT d.display_name
                        FROM discovered_devices d
                        JOIN discovery_observations o ON o.discovered_device_id = d.id
                        WHERE o.identity_id = i.id AND d.display_name IS NOT NULL
                        ORDER BY o.observed_at DESC, o.id DESC
                        LIMIT 1
                    ),
                    i.stable_key,
                    i.id
                 )
                 FROM device_identities i
                 LEFT JOIN device_user_intent ui ON ui.identity_id = i.id
                 WHERE i.id = ?1",
                params![identity_id],
                |row| row.get(0),
            )
            .context("building alias seed")
    }

    fn unique_alias(&self, identity_id: &str, base: &str) -> Result<String> {
        if base.is_empty() {
            bail!("alias cannot be empty after normalization");
        }

        for suffix in 0..1000 {
            let candidate = if suffix == 0 {
                base.to_string()
            } else {
                format!("{base}-{suffix}")
            };
            let conflict: Option<String> = self
                .conn
                .query_row(
                    "SELECT identity_id FROM device_user_intent WHERE alias = ?1 AND identity_id != ?2",
                    params![candidate, identity_id],
                    |row| row.get(0),
                )
                .optional()?;
            if conflict.is_none() {
                return Ok(candidate);
            }
        }

        bail!("could not allocate unique alias for {base}")
    }

    fn mutation_result(&self, identity_id: &str, message: &str) -> Result<DeviceMutationResult> {
        let record = self
            .device_identity_record(identity_id)?
            .context("updated identity disappeared")?;
        Ok(DeviceMutationResult {
            identity: record.identity,
            endpoint_count: record.endpoint_count,
            message: message.to_string(),
        })
    }

    pub fn metadata_value(&self, key: &str) -> Result<Option<String>> {
        self.conn
            .query_row(
                "SELECT value FROM daemon_metadata WHERE key = ?1",
                params![key],
                |row| row.get(0),
            )
            .optional()
            .context("reading daemon metadata")
    }

    fn tags_for_identity(
        &self,
        identity_id: &str,
    ) -> std::result::Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        let mut stmt = self
            .conn
            .prepare("SELECT tag FROM device_tags WHERE identity_id = ?1 ORDER BY tag")?;
        let rows = stmt.query_map(params![identity_id], |row| row.get::<_, String>(0))?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityLookup {
    Found(String),
    NotFound,
    Ambiguous(Vec<String>),
}

fn should_export_device_settings(identity: &DeviceIdentity) -> bool {
    identity.tracked_state != TrackedState::Untracked
        || identity.label.is_some()
        || identity.alias.is_some()
        || identity.category.is_some()
        || !identity.tags.is_empty()
        || identity.ssh_username.is_some()
        || identity.ssh_port.is_some()
        || identity.endpoint_preference != EndpointPreference::Auto
}

fn normalize_mac(value: &str) -> String {
    let parts = value
        .trim()
        .to_ascii_lowercase()
        .replace('-', ":")
        .split(':')
        .filter(|part| !part.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();

    if parts.len() == 6 {
        let parsed = parts
            .iter()
            .map(|part| u8::from_str_radix(part, 16))
            .collect::<std::result::Result<Vec<_>, _>>();
        if let Ok(bytes) = parsed {
            return bytes
                .into_iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<Vec<_>>()
                .join(":");
        }
    }

    parts.join(":")
}

fn normalize_hostname(value: &str) -> String {
    value.trim().trim_end_matches('.').to_ascii_lowercase()
}

fn normalize_tag(value: &str) -> Result<String> {
    let tag = value.trim().to_ascii_lowercase();
    if tag.is_empty() {
        bail!("tag cannot be empty");
    }
    Ok(tag)
}

fn slug_alias(value: &str) -> String {
    let mut output = String::new();
    let mut last_was_dash = false;

    for ch in value.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            output.push(ch);
            last_was_dash = false;
        } else if !last_was_dash && !output.is_empty() {
            output.push('-');
            last_was_dash = true;
        }
    }

    while output.ends_with('-') {
        output.pop();
    }

    if output.is_empty() {
        "device".to_string()
    } else {
        output
    }
}

fn merged_family_ids(
    tx: &rusqlite::Transaction<'_>,
    source_identity_id: &str,
) -> Result<Vec<String>> {
    let mut stmt = tx.prepare(
        "SELECT id FROM device_identities WHERE id = ?1 OR merged_into_identity_id = ?1 ORDER BY id",
    )?;
    let rows = stmt.query_map(params![source_identity_id], |row| row.get::<_, String>(0))?;
    let ids = rows.collect::<std::result::Result<Vec<_>, _>>()?;
    if ids.is_empty() {
        bail!("identity '{source_identity_id}' was not found");
    }
    Ok(ids)
}

fn merge_user_intent(
    tx: &rusqlite::Transaction<'_>,
    source_identity_id: &str,
    target_identity_id: &str,
) -> Result<()> {
    let source = read_intent_snapshot(tx, source_identity_id)?;
    let target = read_intent_snapshot(tx, target_identity_id)?;
    if source.is_none() && target.is_none() {
        return Ok(());
    }

    let merged = merge_intent_snapshots(source, target);
    tx.execute(
        "DELETE FROM device_user_intent WHERE identity_id = ?1",
        params![source_identity_id],
    )?;
    tx.execute(
        "INSERT INTO device_user_intent(
            identity_id, tracked_state, label, alias, category, ssh_username, ssh_port, endpoint_preference
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(identity_id) DO UPDATE SET
            tracked_state = excluded.tracked_state,
            label = excluded.label,
            alias = excluded.alias,
            category = excluded.category,
            ssh_username = excluded.ssh_username,
            ssh_port = excluded.ssh_port,
            endpoint_preference = excluded.endpoint_preference,
            updated_at = CURRENT_TIMESTAMP",
        params![
            target_identity_id,
            merged.tracked_state,
            merged.label,
            merged.alias,
            merged.category,
            merged.ssh_username,
            merged.ssh_port,
            merged.endpoint_preference,
        ],
    )?;
    Ok(())
}

fn read_intent_snapshot(
    tx: &rusqlite::Transaction<'_>,
    identity_id: &str,
) -> Result<Option<IntentSnapshot>> {
    tx.query_row(
        "SELECT tracked_state, label, alias, category, ssh_username, ssh_port, endpoint_preference
         FROM device_user_intent
         WHERE identity_id = ?1",
        params![identity_id],
        |row| {
            Ok(IntentSnapshot {
                tracked_state: row.get(0)?,
                label: row.get(1)?,
                alias: row.get(2)?,
                category: row.get(3)?,
                ssh_username: row.get(4)?,
                ssh_port: row.get(5)?,
                endpoint_preference: row.get(6)?,
            })
        },
    )
    .optional()
    .context("reading device user intent")
}

fn merge_intent_snapshots(
    source: Option<IntentSnapshot>,
    target: Option<IntentSnapshot>,
) -> IntentSnapshot {
    let source = source.unwrap_or_else(default_intent_snapshot);
    let target = target.unwrap_or_else(default_intent_snapshot);
    let tracked_state = if target.tracked_state == "untracked" {
        source.tracked_state
    } else {
        target.tracked_state
    };

    IntentSnapshot {
        tracked_state,
        label: target.label.or(source.label),
        alias: target.alias.or(source.alias),
        category: target.category.or(source.category),
        ssh_username: target.ssh_username.or(source.ssh_username),
        ssh_port: target.ssh_port.or(source.ssh_port),
        endpoint_preference: target.endpoint_preference.or(source.endpoint_preference),
    }
}

fn default_intent_snapshot() -> IntentSnapshot {
    IntentSnapshot {
        tracked_state: "untracked".to_string(),
        label: None,
        alias: None,
        category: None,
        ssh_username: None,
        ssh_port: None,
        endpoint_preference: None,
    }
}

fn merge_endpoints(
    tx: &rusqlite::Transaction<'_>,
    source_identity_id: &str,
    target_identity_id: &str,
) -> Result<()> {
    let endpoints = {
        let mut stmt = tx.prepare(
            "SELECT id, kind, address, port FROM network_endpoints WHERE identity_id = ?1",
        )?;
        let rows = stmt.query_map(params![source_identity_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<i64>>(3)?,
            ))
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()?
    };

    for (endpoint_id, kind, address, port) in endpoints {
        let conflict: Option<String> = tx
            .query_row(
                "SELECT id FROM network_endpoints WHERE identity_id = ?1 AND kind = ?2 AND address = ?3 AND COALESCE(port, 0) = COALESCE(?4, 0)",
                params![target_identity_id, kind, address, port],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(conflict_id) = conflict {
            tx.execute(
                "UPDATE network_endpoints
                 SET hostname = COALESCE(hostname, (SELECT hostname FROM network_endpoints WHERE id = ?2)),
                     reachable_state = CASE
                        WHEN reachable_state = 'online'
                          OR (SELECT reachable_state FROM network_endpoints WHERE id = ?2) = 'online' THEN 'online'
                        WHEN reachable_state = 'unknown' THEN (SELECT reachable_state FROM network_endpoints WHERE id = ?2)
                        ELSE reachable_state
                     END,
                     ssh_capability_state = CASE
                        WHEN ssh_capability_state = 'online'
                          OR (SELECT ssh_capability_state FROM network_endpoints WHERE id = ?2) = 'online' THEN 'online'
                        WHEN ssh_capability_state = 'unknown' THEN (SELECT ssh_capability_state FROM network_endpoints WHERE id = ?2)
                        ELSE ssh_capability_state
                     END,
                     last_seen_at = COALESCE(
                        MAX(last_seen_at, (SELECT last_seen_at FROM network_endpoints WHERE id = ?2)),
                        last_seen_at,
                        (SELECT last_seen_at FROM network_endpoints WHERE id = ?2)
                     ),
                     last_checked_at = COALESCE(
                        MAX(last_checked_at, (SELECT last_checked_at FROM network_endpoints WHERE id = ?2)),
                        last_checked_at,
                        (SELECT last_checked_at FROM network_endpoints WHERE id = ?2)
                     ),
                     updated_at = CURRENT_TIMESTAMP
                 WHERE id = ?1",
                params![conflict_id, endpoint_id],
            )?;
            tx.execute(
                "DELETE FROM network_endpoints WHERE id = ?1",
                params![endpoint_id],
            )?;
        } else {
            tx.execute(
                "UPDATE network_endpoints SET identity_id = ?2, updated_at = CURRENT_TIMESTAMP WHERE id = ?1",
                params![endpoint_id, target_identity_id],
            )?;
        }
    }

    Ok(())
}

fn move_split_endpoints(
    tx: &rusqlite::Transaction<'_>,
    old_identity_id: &str,
    new_identity_id: &str,
    discovered_device_id: &str,
) -> Result<()> {
    let observations = {
        let mut stmt = tx.prepare(
            "SELECT source, evidence_json
             FROM discovery_observations
             WHERE discovered_device_id = ?1
             ORDER BY observed_at, id",
        )?;
        let rows = stmt.query_map(params![discovered_device_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()?
    };

    let mut addresses = Vec::new();
    let mut hostnames = Vec::new();
    for (source, evidence_json) in observations {
        collect_endpoint_keys_from_evidence(
            &source,
            &evidence_json,
            &mut addresses,
            &mut hostnames,
        )?;
    }
    addresses.sort();
    addresses.dedup();
    hostnames.sort();
    hostnames.dedup();

    for address in addresses.iter().filter(|value| !value.is_empty()) {
        tx.execute(
            "UPDATE network_endpoints SET identity_id = ?3, updated_at = CURRENT_TIMESTAMP WHERE identity_id = ?1 AND address = ?2",
            params![old_identity_id, address, new_identity_id],
        )?;
    }
    for hostname in hostnames.iter().filter(|value| !value.is_empty()) {
        tx.execute(
            "UPDATE network_endpoints SET identity_id = ?3, updated_at = CURRENT_TIMESTAMP WHERE identity_id = ?1 AND hostname = ?2",
            params![old_identity_id, hostname, new_identity_id],
        )?;
    }

    Ok(())
}

fn collect_endpoint_keys_from_evidence(
    source: &str,
    evidence_json: &str,
    addresses: &mut Vec<String>,
    hostnames: &mut Vec<String>,
) -> Result<()> {
    match source {
        "arp" => {
            let observation: LanDeviceObservation =
                serde_json::from_str(evidence_json).context("parsing LAN split evidence")?;
            addresses.push(observation.ip_address);
            if let Some(hostname) = observation.hostname {
                addresses.push(hostname.clone());
                hostnames.push(hostname);
            }
        }
        "tailscale" => {
            let observation: TailscaleNodeObservation =
                serde_json::from_str(evidence_json).context("parsing Tailscale split evidence")?;
            addresses.extend(observation.tailscale_ips);
            if let Some(dns_name) = observation.dns_name {
                addresses.push(dns_name.clone());
                hostnames.push(dns_name);
            }
        }
        "mdns" => {
            let observation: MdnsServiceObservation =
                serde_json::from_str(evidence_json).context("parsing mDNS split evidence")?;
            if let Some(hostname) = observation.hostname {
                let hostname = normalize_hostname(&hostname);
                addresses.push(hostname.clone());
                hostnames.push(hostname);
            }
        }
        _ => {}
    }
    Ok(())
}

fn endpoint_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<NetworkEndpoint> {
    let kind_text: String = row.get(2)?;
    let reachability_text: String = row.get(6)?;
    let ssh_capability_text: String = row.get(7)?;
    let port: Option<i64> = row.get(4)?;

    Ok(NetworkEndpoint {
        id: row.get(0)?,
        identity_id: row.get(1)?,
        kind: parse_row_enum(2, &kind_text)?,
        address: row.get(3)?,
        port: parse_optional_port(4, port)?,
        hostname: row.get(5)?,
        reachability: parse_row_enum(6, &reachability_text)?,
        ssh_capability: parse_row_enum(7, &ssh_capability_text)?,
        last_seen_at: row.get(8)?,
        last_checked_at: row.get(9)?,
    })
}

fn parse_row_enum<T>(column_index: usize, value: &str) -> rusqlite::Result<T>
where
    T: FromStr,
    T::Err: Error + Send + Sync + 'static,
{
    T::from_str(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(column_index, Type::Text, Box::new(error))
    })
}

fn parse_optional_port(column_index: usize, value: Option<i64>) -> rusqlite::Result<Option<u16>> {
    match value {
        Some(port) if port > 0 => u16::try_from(port).map(Some).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(column_index, Type::Integer, Box::new(error))
        }),
        Some(_) => Err(rusqlite::Error::FromSqlConversionFailure(
            column_index,
            Type::Integer,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "port must be between 1 and 65535",
            )),
        )),
        None => Ok(None),
    }
}

fn parse_nonnegative_count(column_index: usize, value: i64) -> rusqlite::Result<usize> {
    usize::try_from(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(column_index, Type::Integer, Box::new(error))
    })
}

pub fn default_db_path() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));
    home.join("Library")
        .join("Application Support")
        .join("Network Manager")
        .join("network-manager.sqlite")
}

fn now_timestamp() -> String {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => duration.as_secs().to_string(),
        Err(_) => "0".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use network_manager_core::EndpointKind;

    #[test]
    fn migration_creates_empty_store() {
        let tempdir = tempfile::tempdir().unwrap();
        let path = tempdir.path().join("test.sqlite");
        let store = SqliteStore::open(&path).unwrap();
        store.migrate().unwrap();

        assert!(store.list_device_identities().unwrap().is_empty());
        assert!(store.list_discovered_devices().unwrap().is_empty());
    }

    #[test]
    fn migration_upgrades_pre_identity_correction_schema() {
        let tempdir = tempfile::tempdir().unwrap();
        let path = tempdir.path().join("test.sqlite");
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(
            "CREATE TABLE schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE device_identities (
                id TEXT PRIMARY KEY,
                stable_key TEXT NOT NULL UNIQUE,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE discovered_devices (
                id TEXT PRIMARY KEY,
                source TEXT NOT NULL,
                source_device_id TEXT NOT NULL,
                display_name TEXT,
                raw_json TEXT NOT NULL DEFAULT '{}',
                first_seen_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                last_seen_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                UNIQUE (source, source_device_id)
            );",
        )
        .unwrap();
        drop(conn);

        let store = SqliteStore::open(&path).unwrap();
        store.migrate().unwrap();

        assert!(store
            .table_has_column("device_identities", "merged_into_identity_id")
            .unwrap());
        assert!(store
            .table_has_column("discovered_devices", "identity_override_id")
            .unwrap());
    }

    #[test]
    fn can_insert_and_find_test_identity() {
        let tempdir = tempfile::tempdir().unwrap();
        let path = tempdir.path().join("test.sqlite");
        let store = SqliteStore::open(&path).unwrap();
        store.migrate().unwrap();
        let id = store
            .insert_test_identity("tailscale:node:abc", Some("office-macbook"))
            .unwrap();

        assert_eq!(
            store.find_identity_id("office-macbook").unwrap(),
            IdentityLookup::Found(id)
        );
        assert_eq!(store.list_device_identities().unwrap().len(), 1);
    }

    #[test]
    fn records_tailscale_nodes_as_identities_and_endpoints() {
        let tempdir = tempfile::tempdir().unwrap();
        let path = tempdir.path().join("test.sqlite");
        let store = SqliteStore::open(&path).unwrap();
        store.migrate().unwrap();

        store
            .record_tailscale_nodes(
                Some("example.ts.net"),
                &[TailscaleNodeObservation {
                    source_device_id: "node-1".to_string(),
                    display_name: Some("office-mac".to_string()),
                    dns_name: Some("office-mac.example.ts.net".to_string()),
                    tailscale_ips: vec!["100.64.0.1".to_string()],
                    online: Some(true),
                    os: Some("macOS".to_string()),
                    raw_json: "{}".to_string(),
                }],
            )
            .unwrap();

        let identities = store.list_device_identities().unwrap();
        assert_eq!(identities.len(), 1);
        let endpoints = store
            .endpoints_for_identity(&identities[0].identity.id)
            .unwrap();
        assert_eq!(endpoints.len(), 2);
    }

    #[test]
    fn records_lan_devices_and_tracks_with_generated_alias() {
        let tempdir = tempfile::tempdir().unwrap();
        let path = tempdir.path().join("test.sqlite");
        let store = SqliteStore::open(&path).unwrap();
        store.migrate().unwrap();

        store
            .record_lan_devices(&[LanDeviceObservation {
                ip_address: "192.168.1.10".to_string(),
                hostname: Some("Office MacBook".to_string()),
                mac_address: Some("AA:BB:CC:00:11:22".to_string()),
                interface_name: Some("en0".to_string()),
                raw_text: "Office MacBook (192.168.1.10) at aa:bb:cc:00:11:22 on en0".to_string(),
            }])
            .unwrap();

        let lookup = store.find_identity_id("Office MacBook").unwrap();
        let IdentityLookup::Found(id) = lookup else {
            panic!("expected one LAN identity, got {lookup:?}");
        };

        let result = store
            .set_tracked_state_by_id(&id, TrackedState::Tracked, None, None)
            .unwrap();
        assert_eq!(result.identity.tracked_state, TrackedState::Tracked);
        assert_eq!(result.identity.alias.as_deref(), Some("office-macbook"));
    }

    #[test]
    fn records_mdns_services_as_local_ssh_endpoints() {
        let tempdir = tempfile::tempdir().unwrap();
        let path = tempdir.path().join("test.sqlite");
        let store = SqliteStore::open(&path).unwrap();
        store.migrate().unwrap();

        store
            .record_mdns_services(&[MdnsServiceObservation {
                source_device_id: "local.:_ssh._tcp.:Office Mac".to_string(),
                service_name: "Office Mac".to_string(),
                service_type: "_ssh._tcp".to_string(),
                domain: "local".to_string(),
                hostname: Some("office-mac.local".to_string()),
                ip_addresses: Vec::new(),
                port: Some(22),
                raw_text: "Office Mac._ssh._tcp.local. can be reached at office-mac.local.:22"
                    .to_string(),
            }])
            .unwrap();

        let identities = store.list_device_identities().unwrap();
        assert_eq!(identities.len(), 1);
        let endpoints = store
            .endpoints_for_identity(&identities[0].identity.id)
            .unwrap();
        assert_eq!(endpoints.len(), 1);
        assert_eq!(endpoints[0].kind, EndpointKind::Mdns);
        assert_eq!(endpoints[0].hostname.as_deref(), Some("office-mac.local"));
        assert_eq!(endpoints[0].port, Some(22));
    }

    #[test]
    fn records_mdns_resolved_ips_on_same_identity() {
        let tempdir = tempfile::tempdir().unwrap();
        let path = tempdir.path().join("test.sqlite");
        let store = SqliteStore::open(&path).unwrap();
        store.migrate().unwrap();

        store
            .record_mdns_services(&[MdnsServiceObservation {
                source_device_id: "local.:_ssh._tcp.:Office Mac".to_string(),
                service_name: "Office Mac".to_string(),
                service_type: "_ssh._tcp".to_string(),
                domain: "local".to_string(),
                hostname: Some("office-mac.local".to_string()),
                ip_addresses: vec!["192.168.1.10".to_string()],
                port: Some(22),
                raw_text: "Office Mac._ssh._tcp.local. can be reached at office-mac.local.:22"
                    .to_string(),
            }])
            .unwrap();

        let identities = store.list_device_identities().unwrap();
        assert_eq!(identities.len(), 1);
        let endpoints = store
            .endpoints_for_identity(&identities[0].identity.id)
            .unwrap();
        assert_eq!(endpoints.len(), 2);
        assert!(endpoints.iter().any(|endpoint| {
            endpoint.kind == EndpointKind::Mdns
                && endpoint.address == "office-mac.local"
                && endpoint.port == Some(22)
        }));
        assert!(endpoints.iter().any(|endpoint| {
            endpoint.kind == EndpointKind::LanIp
                && endpoint.address == "192.168.1.10"
                && endpoint.port == Some(22)
        }));
    }

    #[test]
    fn records_existing_dns_endpoint_resolved_ips_on_same_identity() {
        let tempdir = tempfile::tempdir().unwrap();
        let path = tempdir.path().join("test.sqlite");
        let store = SqliteStore::open(&path).unwrap();
        store.migrate().unwrap();

        store
            .record_mdns_services(&[MdnsServiceObservation {
                source_device_id: "local.:_ssh._tcp.:Office Mac".to_string(),
                service_name: "Office Mac".to_string(),
                service_type: "_ssh._tcp".to_string(),
                domain: "local".to_string(),
                hostname: Some("office-mac.local".to_string()),
                ip_addresses: Vec::new(),
                port: Some(22),
                raw_text: "Office Mac._ssh._tcp.local. can be reached at office-mac.local.:22"
                    .to_string(),
            }])
            .unwrap();

        let identity_id = store.list_device_identities().unwrap()[0]
            .identity
            .id
            .clone();
        let mdns_endpoint = store.endpoints_for_identity(&identity_id).unwrap()[0].clone();
        let changed = store
            .record_resolved_endpoint_ips(&mdns_endpoint, &["192.168.1.20".to_string()])
            .unwrap();

        assert_eq!(changed, 1);
        let endpoints = store.endpoints_for_identity(&identity_id).unwrap();
        assert!(endpoints.iter().any(|endpoint| {
            endpoint.kind == EndpointKind::LanIp
                && endpoint.address == "192.168.1.20"
                && endpoint.port == Some(22)
        }));
    }

    #[test]
    fn records_non_ssh_mdns_services_as_online_device_endpoints() {
        let tempdir = tempfile::tempdir().unwrap();
        let path = tempdir.path().join("test.sqlite");
        let store = SqliteStore::open(&path).unwrap();
        store.migrate().unwrap();

        store
            .record_mdns_services(&[MdnsServiceObservation {
                source_device_id: "local.:_workstation._tcp.:homeassistant".to_string(),
                service_name: "homeassistant".to_string(),
                service_type: "_workstation._tcp".to_string(),
                domain: "local".to_string(),
                hostname: Some("homeassistant.local".to_string()),
                ip_addresses: Vec::new(),
                port: None,
                raw_text:
                    "homeassistant._workstation._tcp.local. can be reached at homeassistant.local."
                        .to_string(),
            }])
            .unwrap();

        let identities = store.list_device_identities().unwrap();
        assert_eq!(identities.len(), 1);
        let endpoints = store
            .endpoints_for_identity(&identities[0].identity.id)
            .unwrap();
        assert_eq!(endpoints.len(), 1);
        assert_eq!(endpoints[0].kind, EndpointKind::Mdns);
        assert_eq!(
            endpoints[0].hostname.as_deref(),
            Some("homeassistant.local")
        );
        assert_eq!(endpoints[0].reachability, AvailabilityState::Online);
        assert_eq!(endpoints[0].ssh_capability, AvailabilityState::Unknown);
    }

    #[test]
    fn mdns_unresolved_then_resolved_service_keeps_identity() {
        let tempdir = tempfile::tempdir().unwrap();
        let path = tempdir.path().join("test.sqlite");
        let store = SqliteStore::open(&path).unwrap();
        store.migrate().unwrap();
        let source_device_id = "local.:_ssh._tcp.:Office Mac".to_string();

        store
            .record_mdns_services(&[MdnsServiceObservation {
                source_device_id: source_device_id.clone(),
                service_name: "Office Mac".to_string(),
                service_type: "_ssh._tcp".to_string(),
                domain: "local".to_string(),
                hostname: None,
                ip_addresses: Vec::new(),
                port: None,
                raw_text: "Office Mac._ssh._tcp.local.".to_string(),
            }])
            .unwrap();
        let first_identity = store
            .list_device_identities()
            .unwrap()
            .remove(0)
            .identity
            .id;

        store
            .record_mdns_services(&[MdnsServiceObservation {
                source_device_id,
                service_name: "Office Mac".to_string(),
                service_type: "_ssh._tcp".to_string(),
                domain: "local".to_string(),
                hostname: Some("office-mac.local".to_string()),
                ip_addresses: Vec::new(),
                port: Some(22),
                raw_text: "Office Mac._ssh._tcp.local. can be reached at office-mac.local.:22"
                    .to_string(),
            }])
            .unwrap();

        let identities = store.list_device_identities().unwrap();
        assert_eq!(identities.len(), 1);
        assert_eq!(identities[0].identity.id, first_identity);
        assert_eq!(
            store
                .endpoints_for_identity(&identities[0].identity.id)
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn exports_and_imports_user_settings_for_matching_stable_keys() {
        let source_dir = tempfile::tempdir().unwrap();
        let source_path = source_dir.path().join("source.sqlite");
        let source_store = SqliteStore::open(&source_path).unwrap();
        source_store.migrate().unwrap();
        let source_id = source_store
            .insert_test_identity("tailscale:portable-node", Some("portable"))
            .unwrap();
        source_store
            .set_label_by_id(&source_id, "Portable Mac")
            .unwrap();
        source_store
            .set_category_by_id(&source_id, Some("laptop"))
            .unwrap();
        source_store.add_tag_by_id(&source_id, "Agents").unwrap();
        source_store
            .set_ssh_username_by_id(&source_id, Some("agent"))
            .unwrap();
        source_store
            .set_ssh_port_by_id(&source_id, Some(2222))
            .unwrap();
        source_store
            .set_endpoint_preference_by_id(&source_id, EndpointPreference::LocalFirst)
            .unwrap();

        let export = source_store.export_user_settings().unwrap();
        assert_eq!(export.devices.len(), 1);

        let target_dir = tempfile::tempdir().unwrap();
        let target_path = target_dir.path().join("target.sqlite");
        let target_store = SqliteStore::open(&target_path).unwrap();
        target_store.migrate().unwrap();
        let target_id = target_store
            .insert_test_identity("tailscale:portable-node", None)
            .unwrap();

        let result = target_store.import_user_settings(&export, false).unwrap();
        assert_eq!(result.devices_applied, 1);
        assert_eq!(result.devices_missing, 0);

        let details = target_store
            .device_details_by_id(&target_id)
            .unwrap()
            .unwrap();
        assert_eq!(details.identity.tracked_state, TrackedState::Tracked);
        assert_eq!(details.identity.label.as_deref(), Some("Portable Mac"));
        assert_eq!(details.identity.alias.as_deref(), Some("portable"));
        assert_eq!(details.identity.category.as_deref(), Some("laptop"));
        assert_eq!(details.identity.tags, vec!["agents".to_string()]);
        assert_eq!(details.identity.ssh_username.as_deref(), Some("agent"));
        assert_eq!(details.identity.ssh_port, Some(2222));
        assert_eq!(
            details.identity.endpoint_preference,
            EndpointPreference::LocalFirst
        );
    }

    #[test]
    fn merge_hides_source_and_redirects_lookup() {
        let tempdir = tempfile::tempdir().unwrap();
        let path = tempdir.path().join("test.sqlite");
        let store = SqliteStore::open(&path).unwrap();
        store.migrate().unwrap();
        let source = store
            .insert_test_identity("manual:source", Some("source"))
            .unwrap();
        let target = store
            .insert_test_identity("manual:target", Some("target"))
            .unwrap();

        store
            .merge_identities_by_id(&source, &target, Some("test merge"))
            .unwrap();

        assert_eq!(
            store.find_identity_id(&source).unwrap(),
            IdentityLookup::Found(target.clone())
        );
        assert_eq!(
            store.find_identity_id("manual:source").unwrap(),
            IdentityLookup::Found(target)
        );
        assert_eq!(store.list_device_identities().unwrap().len(), 1);
    }

    #[test]
    fn exports_and_imports_merge_corrections() {
        let source_dir = tempfile::tempdir().unwrap();
        let source_path = source_dir.path().join("source.sqlite");
        let source_store = SqliteStore::open(&source_path).unwrap();
        source_store.migrate().unwrap();
        let source_id = source_store
            .insert_test_identity("manual:merge-source", Some("merge-source"))
            .unwrap();
        let target_id = source_store
            .insert_test_identity("manual:merge-target", Some("merge-target"))
            .unwrap();
        source_store
            .merge_identities_by_id(&source_id, &target_id, Some("portable merge"))
            .unwrap();
        let export = source_store.export_user_settings().unwrap();
        assert_eq!(export.merges.len(), 1);

        let target_dir = tempfile::tempdir().unwrap();
        let target_path = target_dir.path().join("target.sqlite");
        let target_store = SqliteStore::open(&target_path).unwrap();
        target_store.migrate().unwrap();
        target_store
            .insert_test_identity("manual:merge-source", Some("source"))
            .unwrap();
        let imported_target_id = target_store
            .insert_test_identity("manual:merge-target", Some("target"))
            .unwrap();

        let result = target_store.import_user_settings(&export, false).unwrap();
        assert_eq!(result.merges_applied, 1);
        assert_eq!(target_store.list_device_identities().unwrap().len(), 1);
        assert_eq!(
            target_store
                .find_identity_id("manual:merge-source")
                .unwrap(),
            IdentityLookup::Found(imported_target_id)
        );
    }

    #[test]
    fn merge_transfers_source_intent_when_target_has_no_alias() {
        let tempdir = tempfile::tempdir().unwrap();
        let path = tempdir.path().join("test.sqlite");
        let store = SqliteStore::open(&path).unwrap();
        store.migrate().unwrap();
        let source = store
            .insert_test_identity("manual:source-intent", Some("source-alias"))
            .unwrap();
        let target = format!("identity-{}", Uuid::new_v4());
        store
            .conn
            .execute(
                "INSERT INTO device_identities(id, stable_key) VALUES (?1, 'manual:target-intent')",
                params![target],
            )
            .unwrap();
        store
            .conn
            .execute(
                "INSERT INTO device_user_intent(identity_id, tracked_state) VALUES (?1, 'untracked')",
                params![target],
            )
            .unwrap();

        store
            .merge_identities_by_id(&source, &target, Some("test merge"))
            .unwrap();

        let details = store.device_details_by_id(&target).unwrap().unwrap();
        assert_eq!(details.identity.tracked_state, TrackedState::Tracked);
        assert_eq!(details.identity.alias.as_deref(), Some("source-alias"));
    }

    #[test]
    fn merge_target_intent_wins_conflicts_and_tags_union() {
        let tempdir = tempfile::tempdir().unwrap();
        let path = tempdir.path().join("test.sqlite");
        let store = SqliteStore::open(&path).unwrap();
        store.migrate().unwrap();
        let source = store
            .insert_test_identity("manual:source-conflict", Some("source-alias"))
            .unwrap();
        store
            .set_category_by_id(&source, Some("source-cat"))
            .unwrap();
        store
            .set_ssh_username_by_id(&source, Some("source-user"))
            .unwrap();
        store.set_ssh_port_by_id(&source, Some(2200)).unwrap();
        store
            .set_endpoint_preference_by_id(&source, EndpointPreference::LocalFirst)
            .unwrap();
        store.add_tag_by_id(&source, "source-tag").unwrap();

        let target = store
            .insert_test_identity("manual:target-conflict", Some("target-alias"))
            .unwrap();
        store
            .set_tracked_state_by_id(&target, TrackedState::Ignored, None, None)
            .unwrap();
        store
            .set_category_by_id(&target, Some("target-cat"))
            .unwrap();
        store
            .set_ssh_username_by_id(&target, Some("target-user"))
            .unwrap();
        store.set_ssh_port_by_id(&target, Some(2222)).unwrap();
        store
            .set_endpoint_preference_by_id(&target, EndpointPreference::TailscaleFirst)
            .unwrap();
        store.add_tag_by_id(&target, "target-tag").unwrap();

        store
            .merge_identities_by_id(&source, &target, None)
            .unwrap();

        let details = store.device_details_by_id(&target).unwrap().unwrap();
        assert_eq!(details.identity.tracked_state, TrackedState::Ignored);
        assert_eq!(details.identity.alias.as_deref(), Some("target-alias"));
        assert_eq!(details.identity.category.as_deref(), Some("target-cat"));
        assert_eq!(
            details.identity.ssh_username.as_deref(),
            Some("target-user")
        );
        assert_eq!(details.identity.ssh_port, Some(2222));
        assert_eq!(
            details.identity.endpoint_preference,
            EndpointPreference::TailscaleFirst
        );
        assert_eq!(details.identity.tags, vec!["source-tag", "target-tag"]);
    }

    #[test]
    fn merge_chains_resolve_to_final_active_identity() {
        let tempdir = tempfile::tempdir().unwrap();
        let path = tempdir.path().join("test.sqlite");
        let store = SqliteStore::open(&path).unwrap();
        store.migrate().unwrap();
        let first = store
            .insert_test_identity("manual:first", Some("first"))
            .unwrap();
        let second = store.insert_test_identity("manual:second", None).unwrap();
        let third = store.insert_test_identity("manual:third", None).unwrap();

        store.merge_identities_by_id(&first, &second, None).unwrap();
        store.merge_identities_by_id(&second, &third, None).unwrap();

        assert_eq!(
            store.find_identity_id("manual:first").unwrap(),
            IdentityLookup::Found(third.clone())
        );
        assert_eq!(
            store.find_identity_id("manual:second").unwrap(),
            IdentityLookup::Found(third.clone())
        );
        assert_eq!(
            store.find_identity_id("first").unwrap(),
            IdentityLookup::Found(third)
        );
        assert_eq!(store.list_device_identities().unwrap().len(), 1);
    }

    #[test]
    fn rediscovery_after_merge_stays_on_target_identity() {
        let tempdir = tempfile::tempdir().unwrap();
        let path = tempdir.path().join("test.sqlite");
        let store = SqliteStore::open(&path).unwrap();
        store.migrate().unwrap();
        let observation = LanDeviceObservation {
            ip_address: "192.168.1.10".to_string(),
            hostname: Some("Office MacBook".to_string()),
            mac_address: Some("AA:BB:CC:00:11:22".to_string()),
            interface_name: Some("en0".to_string()),
            raw_text: "Office MacBook (192.168.1.10) at aa:bb:cc:00:11:22 on en0".to_string(),
        };
        store
            .record_lan_devices(std::slice::from_ref(&observation))
            .unwrap();
        let source = match store.find_identity_id("mac:aa:bb:cc:00:11:22").unwrap() {
            IdentityLookup::Found(id) => id,
            other => panic!("expected LAN identity, got {other:?}"),
        };
        let target = store
            .insert_test_identity("manual:target-rediscovery", None)
            .unwrap();

        store
            .merge_identities_by_id(&source, &target, None)
            .unwrap();
        store.record_lan_devices(&[observation]).unwrap();

        assert_eq!(
            store.find_identity_id("mac:aa:bb:cc:00:11:22").unwrap(),
            IdentityLookup::Found(target.clone())
        );
        let endpoints = store.endpoints_for_identity(&target).unwrap();
        assert!(endpoints
            .iter()
            .any(|endpoint| endpoint.address == "192.168.1.10"));
        assert_eq!(store.list_device_identities().unwrap().len(), 1);
    }

    #[test]
    fn split_discovered_lan_device_moves_matching_endpoint() {
        let tempdir = tempfile::tempdir().unwrap();
        let path = tempdir.path().join("test.sqlite");
        let store = SqliteStore::open(&path).unwrap();
        store.migrate().unwrap();
        let observation = LanDeviceObservation {
            ip_address: "192.168.1.10".to_string(),
            hostname: Some("Office MacBook".to_string()),
            mac_address: Some("AA:BB:CC:00:11:22".to_string()),
            interface_name: Some("en0".to_string()),
            raw_text: "Office MacBook (192.168.1.10) at aa:bb:cc:00:11:22 on en0".to_string(),
        };
        store
            .record_lan_devices(std::slice::from_ref(&observation))
            .unwrap();
        let mut second_observation = observation.clone();
        second_observation.ip_address = "192.168.1.11".to_string();
        second_observation.raw_text =
            "Office MacBook (192.168.1.11) at aa:bb:cc:00:11:22 on en0".to_string();
        store
            .record_lan_devices(std::slice::from_ref(&second_observation))
            .unwrap();
        let discovered_record = store.list_discovered_devices().unwrap().remove(0);
        let discovered = discovered_record.device.id;
        let old_identity_id = discovered_record.identity_id.unwrap();

        let result = store
            .split_discovered_device_by_id(&discovered, Some("test split"))
            .unwrap();
        store.record_lan_devices(&[observation]).unwrap();
        let endpoints = store.endpoints_for_identity(&result.identity_id).unwrap();
        let old_endpoints = store.endpoints_for_identity(&old_identity_id).unwrap();
        let discovered_record = store
            .list_discovered_devices()
            .unwrap()
            .into_iter()
            .find(|record| record.device.id == discovered)
            .unwrap();
        let stale_evidence_count: i64 = store
            .conn
            .query_row(
                "SELECT COUNT(*) FROM identity_evidence WHERE discovered_device_id = ?1 AND identity_id = ?2",
                params![&discovered, &old_identity_id],
                |row| row.get(0),
            )
            .unwrap();
        let new_evidence_count: i64 = store
            .conn
            .query_row(
                "SELECT COUNT(*) FROM identity_evidence WHERE discovered_device_id = ?1 AND identity_id = ?2",
                params![&discovered, &result.identity_id],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(endpoints.len(), 3);
        assert!(endpoints
            .iter()
            .any(|endpoint| endpoint.address == "192.168.1.10"));
        assert!(endpoints
            .iter()
            .any(|endpoint| endpoint.address == "192.168.1.11"));
        assert!(!old_endpoints.iter().any(
            |endpoint| endpoint.address == "192.168.1.10" || endpoint.address == "192.168.1.11"
        ));
        assert_eq!(
            discovered_record.identity_id,
            Some(result.identity_id.clone())
        );
        assert_eq!(stale_evidence_count, 0);
        assert!(new_evidence_count > 0);
        assert!(store
            .split_discovered_device_by_id(&discovered, Some("again"))
            .is_err());
    }
}
