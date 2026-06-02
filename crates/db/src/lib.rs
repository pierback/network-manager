use anyhow::{bail, Context, Result};
use network_manager_core::{
    AvailabilityState, DeviceIdentity, DiscoveredDevice, EndpointKind, EndpointPreference,
    NetworkEndpoint, TrackedState,
};
use rusqlite::{params, Connection, OptionalExtension};
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
        Ok(())
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
                    tracked_state: TrackedState::from_str(&tracked_state_text)
                        .unwrap_or(TrackedState::Untracked),
                    category: row.get(5)?,
                    tags,
                    ssh_username: row.get(6)?,
                    ssh_port: ssh_port.and_then(|port| u16::try_from(port).ok()),
                    endpoint_preference: EndpointPreference::from_str(&endpoint_preference_text)
                        .unwrap_or(EndpointPreference::Auto),
                    last_seen_at: row.get(9)?,
                },
                endpoint_count: endpoint_count as usize,
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
                (
                    SELECT o.identity_id
                    FROM discovery_observations o
                    WHERE o.discovered_device_id = d.id AND o.identity_id IS NOT NULL
                    ORDER BY o.observed_at DESC, o.id DESC
                    LIMIT 1
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

        rows.collect::<std::result::Result<Vec<_>, _>>()
            .context("listing discovered devices")
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
                SELECT o.identity_id AS id, 3 AS priority
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
        let matches = stmt
            .query_map(params![query], |row| row.get::<_, String>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?;

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
            let identity_id = self.identity_id_for_stable_key(&stable_key)?;
            let discovered_id =
                self.discovered_id_for_source("tailscale", &node.source_device_id)?;
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
            let identity_id = if let Some(mac) = observation
                .mac_address
                .as_deref()
                .map(normalize_mac)
                .filter(|value| !value.is_empty())
            {
                Some(self.identity_id_for_stable_key(&format!("mac:{mac}"))?)
            } else if let Some(hostname) = observation
                .hostname
                .as_deref()
                .filter(|value| !value.is_empty())
            {
                Some(self.identity_id_for_stable_key(&format!("lan-host:{hostname}"))?)
            } else {
                None
            };

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

    fn identity_id_for_stable_key(&self, stable_key: &str) -> Result<String> {
        if let Some(id) = self
            .conn
            .query_row(
                "SELECT id FROM device_identities WHERE stable_key = ?1",
                params![stable_key],
                |row| row.get(0),
            )
            .optional()?
        {
            return Ok(id);
        }

        let id = format!("identity-{}", Uuid::new_v4());
        self.conn.execute(
            "INSERT INTO device_identities(id, stable_key) VALUES (?1, ?2)",
            params![id, stable_key],
        )?;
        Ok(id)
    }

    fn discovered_id_for_source(&self, source: &str, source_device_id: &str) -> Result<String> {
        if let Some(id) = self
            .conn
            .query_row(
                "SELECT id FROM discovered_devices WHERE source = ?1 AND source_device_id = ?2",
                params![source, source_device_id],
                |row| row.get(0),
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

    fn metadata_value(&self, key: &str) -> Result<Option<String>> {
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

fn normalize_mac(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .replace('-', ":")
        .split(':')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(":")
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

fn endpoint_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<NetworkEndpoint> {
    let kind_text: String = row.get(2)?;
    let reachability_text: String = row.get(6)?;
    let ssh_capability_text: String = row.get(7)?;
    let port: Option<i64> = row.get(4)?;

    Ok(NetworkEndpoint {
        id: row.get(0)?,
        identity_id: row.get(1)?,
        kind: EndpointKind::from_str(&kind_text).unwrap_or(EndpointKind::Other),
        address: row.get(3)?,
        port: port.and_then(|port| u16::try_from(port).ok()),
        hostname: row.get(5)?,
        reachability: AvailabilityState::from_str(&reachability_text)
            .unwrap_or(AvailabilityState::Unknown),
        ssh_capability: AvailabilityState::from_str(&ssh_capability_text)
            .unwrap_or(AvailabilityState::Unknown),
        last_seen_at: row.get(8)?,
        last_checked_at: row.get(9)?,
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
}
