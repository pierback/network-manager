use std::path::{Path, PathBuf};

use network_manager_core::{
    resolve_ssh_target, AvailabilityState, DeviceIdentity, EndpointKind, EndpointPreference,
    NetworkEndpoint, TrackedState,
};
use network_manager_db::{
    DeviceDetails, DeviceIdentityRecord, DiscoveredDeviceRecord, SqliteStore,
};

use super::{
    DaemonStatusVm, DashboardVm, DeviceDetailVm, DeviceIdentityVm, DiscoveryRowVm, DiscoveryVm,
    EndpointGroup, EndpointVm, NetworkManagerRepository, QuickAccessVm, SettingsVm, SshTargetVm,
    TrackedDeviceRowVm,
};

#[derive(Debug, Clone)]
pub struct SqliteRepository {
    db_path: PathBuf,
}

impl SqliteRepository {
    pub fn new(db_path: impl AsRef<Path>) -> Self {
        Self {
            db_path: db_path.as_ref().to_path_buf(),
        }
    }

    fn store(&self) -> anyhow::Result<SqliteStore> {
        let store = SqliteStore::open(&self.db_path)?;
        store.migrate()?;
        Ok(store)
    }
}

impl Default for SqliteRepository {
    fn default() -> Self {
        Self::new(network_manager_db::default_db_path())
    }
}

impl NetworkManagerRepository for SqliteRepository {
    fn dashboard(&self) -> DashboardVm {
        let Ok(store) = self.store() else {
            return empty_dashboard("sqlite unavailable");
        };
        let daemon = daemon_vm(&store);
        let tracked = store
            .list_device_identities()
            .unwrap_or_default()
            .into_iter()
            .filter(|record| record.identity.tracked_state == TrackedState::Tracked)
            .map(|record| tracked_row(&store, record))
            .collect::<Vec<_>>();

        DashboardVm {
            online_count: tracked
                .iter()
                .filter(|row| row.overall == AvailabilityState::Online)
                .count(),
            tailscale_count: tracked
                .iter()
                .filter(|row| row.tailscale == AvailabilityState::Online)
                .count(),
            tracked,
            daemon,
        }
    }

    fn discovery(&self) -> DiscoveryVm {
        let Ok(store) = self.store() else {
            return DiscoveryVm {
                rows: Vec::new(),
                filters: default_filters(),
                possible_match: Some("SQLite store unavailable".into()),
            };
        };
        let rows = store
            .list_discovered_devices()
            .unwrap_or_default()
            .into_iter()
            .filter(|record| !is_noisy_discovery(record))
            .map(|record| discovery_row(&store, record))
            .collect();
        DiscoveryVm {
            rows,
            filters: default_filters(),
            possible_match: None,
        }
    }

    fn selected_device_detail(&self, selected_identity_id: Option<&str>) -> DeviceDetailVm {
        let Ok(store) = self.store() else {
            return empty_detail("sqlite unavailable", Vec::new());
        };
        let identities = store.list_device_identities().unwrap_or_default();
        let device_list = identities
            .iter()
            .map(|record| device_identity_vm(&record.identity))
            .collect::<Vec<_>>();
        let selected = selected_identity_id
            .and_then(|identity_id| {
                identities
                    .iter()
                    .find(|record| record.identity.id == identity_id)
            })
            .or_else(|| {
                identities
                    .iter()
                    .find(|record| record.identity.tracked_state == TrackedState::Tracked)
            })
            .or_else(|| identities.first());
        let Some(selected) = selected else {
            return empty_detail("No devices discovered yet", Vec::new());
        };
        store
            .device_details_by_id(&selected.identity.id)
            .ok()
            .flatten()
            .map(|details| device_detail(details, device_list.clone()))
            .unwrap_or_else(|| empty_detail("Device details unavailable", device_list))
    }

    fn quick_access(&self) -> QuickAccessVm {
        let dashboard = self.dashboard();
        QuickAccessVm {
            rows: dashboard.tracked.into_iter().take(4).collect(),
            last_scan: dashboard.daemon.last_scan,
        }
    }

    fn settings(&self) -> SettingsVm {
        let daemon = self
            .store()
            .map(|store| daemon_vm(&store))
            .unwrap_or_else(|_| daemon_status("sqlite unavailable"));
        SettingsVm {
            discovery_interval: "Daemon managed".into(),
            battery_mode: true,
            tailscale_enabled: true,
            tailscale_status: daemon.tailscale_service,
            ssh_config_export: false,
            debug_logging: false,
        }
    }
}

fn daemon_vm(store: &SqliteStore) -> DaemonStatusVm {
    let status =
        store
            .daemon_status("sqlite")
            .unwrap_or_else(|_| network_manager_db::DaemonStatus {
                state: "unknown".into(),
                source: "sqlite".into(),
                started_at: None,
                updated_at: None,
                db_path: None,
                stale: true,
            });
    let tailscale_service = store
        .metadata_value("tailscale_service_state")
        .ok()
        .flatten()
        .as_deref()
        .map(parse_tailscale_service_state)
        .unwrap_or(AvailabilityState::Unknown);
    DaemonStatusVm {
        state: parse_availability(&status.state),
        source: status.source,
        tailscale_service,
        last_scan: status.updated_at.unwrap_or_else(|| "never".into()),
        stale: status.stale,
    }
}

fn daemon_status(source: &str) -> DaemonStatusVm {
    DaemonStatusVm {
        state: AvailabilityState::Unknown,
        source: source.into(),
        tailscale_service: AvailabilityState::Unknown,
        last_scan: "never".into(),
        stale: true,
    }
}

fn empty_dashboard(source: &str) -> DashboardVm {
    DashboardVm {
        daemon: daemon_status(source),
        tracked: Vec::new(),
        online_count: 0,
        tailscale_count: 0,
    }
}

fn default_filters() -> Vec<String> {
    vec!["All sources".into(), "By type".into(), "Untracked".into()]
}

fn tracked_row(store: &SqliteStore, record: DeviceIdentityRecord) -> TrackedDeviceRowVm {
    let endpoints = store
        .endpoints_for_identity(&record.identity.id)
        .unwrap_or_default();
    tracked_row_from_parts(record.identity, endpoints)
}

fn tracked_row_from_parts(
    identity: DeviceIdentity,
    endpoints: Vec<NetworkEndpoint>,
) -> TrackedDeviceRowVm {
    let target = resolve_ssh_target(
        &endpoints,
        identity.endpoint_preference,
        identity.ssh_username.as_deref(),
        identity.ssh_port,
    );
    let preferred = target
        .as_ref()
        .map(|target| ssh_destination(target.username.as_deref(), &target.host, target.port))
        .unwrap_or_else(|| "No SSH capability".into());
    let target_reason = target
        .as_ref()
        .map(|target| format!("Selected {} endpoint", target.endpoint_kind.as_str()))
        .unwrap_or_else(|| "No reachable SSH endpoint is known".into());

    let category = identity
        .category
        .clone()
        .unwrap_or_else(|| infer_device_type_for_identity(&identity, &endpoints));

    TrackedDeviceRowVm {
        id: identity.id.clone(),
        label: identity_label(&identity),
        alias: identity
            .alias
            .clone()
            .unwrap_or_else(|| identity.id.clone()),
        category,
        overall: aggregate_reachability(&endpoints, |_| true),
        lan: aggregate_reachability(&endpoints, EndpointKind::is_lan),
        tailscale: aggregate_reachability(&endpoints, EndpointKind::is_tailscale),
        ssh: aggregate_ssh(&endpoints),
        preferred_target: preferred,
        target_reason,
        last_seen: identity.last_seen_at.unwrap_or_else(|| "unknown".into()),
    }
}

fn is_noisy_discovery(record: &DiscoveredDeviceRecord) -> bool {
    record.device.source == "mdns"
        && (record.device.source_device_id.contains(":_raop._tcp:")
            || record.device.source_device_id.contains(":_airplay._tcp:"))
}

fn discovery_row(store: &SqliteStore, record: DiscoveredDeviceRecord) -> DiscoveryRowVm {
    let details = record
        .identity_id
        .as_ref()
        .and_then(|identity_id| store.device_details_by_id(identity_id).ok().flatten());
    let display_name = record
        .device
        .display_name
        .clone()
        .unwrap_or_else(|| record.device.source_device_id.clone());
    let (tracked_state, category, availability, sources, hostname, ip_address) = details
        .as_ref()
        .map(|details| {
            (
                details.identity.tracked_state,
                details.identity.category.clone().unwrap_or_else(|| {
                    infer_device_type_for_discovery(&record, Some(details), &display_name)
                }),
                aggregate_reachability(&details.endpoints, |_| true),
                discovery_sources(&record, &details.endpoints),
                endpoint_hostname(&details.endpoints, &record.device.source_device_id),
                endpoint_ip_address(&details.endpoints, &record.device.source_device_id),
            )
        })
        .unwrap_or_else(|| {
            (
                TrackedState::Untracked,
                infer_device_type_for_discovery(&record, None, &display_name),
                AvailabilityState::Unknown,
                vec![source_label(&record.device.source).into()],
                record.device.source_device_id.clone(),
                ip_like_or_dash(&record.device.source_device_id),
            )
        });

    DiscoveryRowVm {
        id: record.device.id,
        identity_id: record.identity_id,
        display_name,
        hostname,
        ip_address,
        source: source_label(&record.device.source).into(),
        sources,
        category,
        tracked_state,
        availability,
        last_seen: record.device.last_seen_at,
    }
}

fn device_detail(details: DeviceDetails, device_list: Vec<DeviceIdentityVm>) -> DeviceDetailVm {
    let identity = details.identity;
    let endpoints = details.endpoints;
    let target = resolve_ssh_target(
        &endpoints,
        identity.endpoint_preference,
        identity.ssh_username.as_deref(),
        identity.ssh_port,
    );
    let preferred_endpoint_id = target.as_ref().map(|target| target.endpoint_id.clone());

    DeviceDetailVm {
        identity: device_identity_vm(&identity),
        device_list,
        endpoints: endpoints
            .iter()
            .map(|endpoint| EndpointVm {
                id: endpoint.id.clone(),
                group: endpoint_group(endpoint.kind),
                kind: endpoint.kind,
                address: endpoint.address.clone(),
                reachability: endpoint.reachability,
                ssh_capability: endpoint.ssh_capability,
                last_checked: endpoint
                    .last_checked_at
                    .clone()
                    .unwrap_or_else(|| "never".into()),
                preferred: preferred_endpoint_id
                    .as_deref()
                    .is_some_and(|id| id == endpoint.id),
            })
            .collect(),
        preferred_target: target.map(|target| SshTargetVm {
            destination: ssh_destination(target.username.as_deref(), &target.host, target.port),
            reason: format!("Selected {} endpoint", target.endpoint_kind.as_str()),
        }),
        evidence: vec![format!("Stable key: {}", identity.stable_key)],
    }
}

fn empty_detail(label: &str, device_list: Vec<DeviceIdentityVm>) -> DeviceDetailVm {
    DeviceDetailVm {
        identity: DeviceIdentityVm {
            id: "empty".into(),
            label: label.into(),
            alias: "—".into(),
            category: "Device".into(),
            tracked_state: TrackedState::Untracked,
            ssh_username: None,
            endpoint_preference: EndpointPreference::Auto,
        },
        device_list,
        endpoints: Vec::new(),
        preferred_target: None,
        evidence: Vec::new(),
    }
}

fn device_identity_vm(identity: &DeviceIdentity) -> DeviceIdentityVm {
    DeviceIdentityVm {
        id: identity.id.clone(),
        label: identity_label(identity),
        alias: identity
            .alias
            .clone()
            .unwrap_or_else(|| identity.id.clone()),
        category: identity.category.clone().unwrap_or_else(|| "Device".into()),
        tracked_state: identity.tracked_state,
        ssh_username: identity.ssh_username.clone(),
        endpoint_preference: identity.endpoint_preference,
    }
}

fn infer_device_type_for_discovery(
    record: &DiscoveredDeviceRecord,
    details: Option<&DeviceDetails>,
    display_name: &str,
) -> String {
    let mut values = vec![
        display_name.to_string(),
        record.device.source_device_id.clone(),
        record.device.source.clone(),
    ];
    if let Some(device_type) = infer_gateway_type(display_name) {
        return device_type;
    }
    if let Some(details) = details {
        if let Some(device_type) = details
            .endpoints
            .iter()
            .find_map(|endpoint| infer_gateway_type(&endpoint.address))
        {
            return device_type;
        }
        values.push(details.identity.stable_key.clone());
        values.extend(details.identity.label.iter().cloned());
        values.extend(details.identity.alias.iter().cloned());
        values.extend(details.endpoints.iter().flat_map(|endpoint| {
            [
                endpoint.address.clone(),
                endpoint.hostname.clone().unwrap_or_default(),
                endpoint.kind.as_str().to_string(),
            ]
        }));
    }
    infer_device_type(&values).unwrap_or_else(|| infer_ip_device_type(display_name))
}

fn infer_device_type_for_identity(
    identity: &DeviceIdentity,
    endpoints: &[NetworkEndpoint],
) -> String {
    let mut values = vec![identity.stable_key.clone(), identity.id.clone()];
    values.extend(identity.label.iter().cloned());
    values.extend(identity.alias.iter().cloned());
    if let Some(device_type) = endpoints
        .iter()
        .find_map(|endpoint| infer_gateway_type(&endpoint.address))
    {
        return device_type;
    }
    values.extend(endpoints.iter().flat_map(|endpoint| {
        [
            endpoint.address.clone(),
            endpoint.hostname.clone().unwrap_or_default(),
            endpoint.kind.as_str().to_string(),
        ]
    }));
    infer_device_type(&values).unwrap_or_else(|| "Device".into())
}

fn infer_device_type(values: &[String]) -> Option<String> {
    let text = values.join(" ").to_ascii_lowercase();
    let normalized = text.replace(['_', '-', '.'], " ");

    let checks = [
        (
            "Mac",
            [
                "macbook",
                "mac mini",
                "macmini",
                "imac",
                "mac studio",
                "macstudio",
            ]
            .as_slice(),
        ),
        ("iPhone", ["iphone"].as_slice()),
        ("iPad", ["ipad"].as_slice()),
        ("Apple Watch", ["apple watch", "watch"].as_slice()),
        (
            "Media Device",
            ["apple tv", "appletv", "homepod"].as_slice(),
        ),
        (
            "Printer",
            [
                "printer",
                "laserjet",
                "officejet",
                "_ipp",
                "_ipps",
                "_printer",
            ]
            .as_slice(),
        ),
        (
            "NAS / Storage",
            ["nas", "synology", "diskstation", "qnap", "truenas", "_smb"].as_slice(),
        ),
        (
            "Router / Gateway",
            ["router", "gateway", "fritz", "eero", "unifi", "ubiquiti"].as_slice(),
        ),
        (
            "Computer",
            ["_workstation", "workstation", "desktop", "laptop", "pc"].as_slice(),
        ),
        ("SSH Host", ["_ssh", "ssh"].as_slice()),
        ("Web Device", ["_http", "http"].as_slice()),
    ];

    checks.iter().find_map(|(label, needles)| {
        needles
            .iter()
            .any(|needle| text.contains(needle) || normalized.contains(needle))
            .then(|| (*label).to_string())
    })
}

fn infer_ip_device_type(value: &str) -> String {
    infer_gateway_type(value).unwrap_or_else(|| "Device".into())
}

fn infer_gateway_type(value: &str) -> Option<String> {
    let normalized = value.replace('-', ".");
    let octets = normalized.split('.').collect::<Vec<_>>();
    if octets.len() != 4 || !octets.iter().all(|octet| octet.parse::<u8>().is_ok()) {
        return None;
    }
    let last_octet = octets[3].parse::<u8>().ok()?;
    (last_octet == 1 || last_octet == 254).then(|| "Router / Gateway".into())
}

fn endpoint_group(kind: EndpointKind) -> EndpointGroup {
    if kind.is_lan() {
        EndpointGroup::Lan
    } else if kind.is_tailscale() {
        EndpointGroup::Tailscale
    } else {
        EndpointGroup::Other
    }
}

fn discovery_sources(
    record: &DiscoveredDeviceRecord,
    endpoints: &[NetworkEndpoint],
) -> Vec<String> {
    let mut sources = endpoint_sources(endpoints);
    let source = source_label(&record.device.source).to_string();
    if !sources.iter().any(|existing| existing == &source) {
        sources.push(source);
    }
    if sources.len() > 1 {
        sources.retain(|source| source != "Other");
    }
    sources
}

fn endpoint_sources(endpoints: &[NetworkEndpoint]) -> Vec<String> {
    let mut sources = Vec::new();
    if endpoints.iter().any(|endpoint| endpoint.kind.is_lan()) {
        sources.push("LAN".into());
    }
    if endpoints
        .iter()
        .any(|endpoint| endpoint.kind.is_tailscale())
    {
        sources.push("Tailscale".into());
    }
    if endpoints
        .iter()
        .any(|endpoint| endpoint.kind == EndpointKind::Mdns)
    {
        sources.push("mDNS".into());
    }
    if sources.is_empty() {
        sources.push("Other".into());
    }
    sources
}

fn endpoint_hostname(endpoints: &[NetworkEndpoint], fallback: &str) -> String {
    endpoints
        .iter()
        .find_map(|endpoint| endpoint.hostname.as_deref())
        .filter(|hostname| !hostname.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| {
            if fallback.parse::<std::net::IpAddr>().is_ok() {
                "—".into()
            } else {
                fallback.into()
            }
        })
}

fn endpoint_ip_address(endpoints: &[NetworkEndpoint], fallback: &str) -> String {
    endpoints
        .iter()
        .map(|endpoint| endpoint.address.as_str())
        .find(|address| address.parse::<std::net::IpAddr>().is_ok())
        .map(ToString::to_string)
        .unwrap_or_else(|| ip_like_or_dash(fallback))
}

fn ip_like_or_dash(value: &str) -> String {
    if value.parse::<std::net::IpAddr>().is_ok() {
        value.into()
    } else {
        "—".into()
    }
}

fn aggregate_reachability(
    endpoints: &[NetworkEndpoint],
    predicate: impl Fn(EndpointKind) -> bool,
) -> AvailabilityState {
    aggregate_state(
        endpoints
            .iter()
            .filter(|endpoint| predicate(endpoint.kind))
            .map(|endpoint| endpoint.reachability),
    )
}

fn aggregate_ssh(endpoints: &[NetworkEndpoint]) -> AvailabilityState {
    aggregate_state(endpoints.iter().map(|endpoint| endpoint.ssh_capability))
}

fn aggregate_state(states: impl Iterator<Item = AvailabilityState>) -> AvailabilityState {
    let mut saw_offline = false;
    for state in states {
        match state {
            AvailabilityState::Online => return AvailabilityState::Online,
            AvailabilityState::Offline => saw_offline = true,
            AvailabilityState::Unknown => {}
        }
    }
    if saw_offline {
        AvailabilityState::Offline
    } else {
        AvailabilityState::Unknown
    }
}

fn parse_availability(value: &str) -> AvailabilityState {
    match value {
        "online" => AvailabilityState::Online,
        "offline" => AvailabilityState::Offline,
        _ => AvailabilityState::Unknown,
    }
}

fn parse_tailscale_service_state(value: &str) -> AvailabilityState {
    match value.to_ascii_lowercase().as_str() {
        "running" => AvailabilityState::Online,
        "stopped" | "unavailable" | "error" => AvailabilityState::Offline,
        "starting" | "needslogin" | "needlogin" => AvailabilityState::Unknown,
        _ => AvailabilityState::Unknown,
    }
}

fn identity_label(identity: &DeviceIdentity) -> String {
    identity
        .label
        .clone()
        .or_else(|| identity.alias.clone())
        .unwrap_or_else(|| identity.stable_key.clone())
}

fn ssh_destination(username: Option<&str>, host: &str, port: u16) -> String {
    let destination = username
        .map(|username| format!("{username}@{host}"))
        .unwrap_or_else(|| host.to_string());
    if port == 22 {
        destination
    } else {
        format!("{destination}:{port}")
    }
}

fn source_label(source: &str) -> &'static str {
    match source {
        "arp" => "ARP",
        "mdns" => "mDNS",
        "tailscale" => "Tailscale",
        _ => "Discovery",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use network_manager_db::LanDeviceObservation;

    #[test]
    fn sqlite_repository_dashboard_uses_tracked_devices_only() {
        let tempdir = tempfile::tempdir().unwrap();
        let path = tempdir.path().join("test.sqlite");
        let store = SqliteStore::open(&path).unwrap();
        store.migrate().unwrap();
        store
            .record_lan_devices(&[LanDeviceObservation {
                ip_address: "192.168.1.10".into(),
                hostname: Some("Office MacBook".into()),
                mac_address: Some("AA:BB:CC:00:11:22".into()),
                interface_name: Some("en0".into()),
                raw_text: "Office MacBook (192.168.1.10) at aa:bb:cc:00:11:22 on en0".into(),
            }])
            .unwrap();
        let id = match store.find_identity_id("Office MacBook").unwrap() {
            network_manager_db::IdentityLookup::Found(id) => id,
            other => panic!("expected identity, got {other:?}"),
        };
        store
            .set_tracked_state_by_id(&id, TrackedState::Tracked, None, None)
            .unwrap();

        let dashboard = SqliteRepository::new(&path).dashboard();
        assert_eq!(dashboard.tracked.len(), 1);
        assert_eq!(dashboard.tracked[0].alias, "office-macbook");
    }

    #[test]
    fn sqlite_repository_selects_requested_device_detail() {
        let tempdir = tempfile::tempdir().unwrap();
        let path = tempdir.path().join("test.sqlite");
        let store = SqliteStore::open(&path).unwrap();
        store.migrate().unwrap();
        store
            .record_lan_devices(&[
                LanDeviceObservation {
                    ip_address: "192.168.1.10".into(),
                    hostname: Some("Office MacBook".into()),
                    mac_address: Some("AA:BB:CC:00:11:22".into()),
                    interface_name: Some("en0".into()),
                    raw_text: "Office MacBook (192.168.1.10) at aa:bb:cc:00:11:22 on en0".into(),
                },
                LanDeviceObservation {
                    ip_address: "192.168.1.20".into(),
                    hostname: Some("Lab NAS".into()),
                    mac_address: Some("AA:BB:CC:00:11:33".into()),
                    interface_name: Some("en0".into()),
                    raw_text: "Lab NAS (192.168.1.20) at aa:bb:cc:00:11:33 on en0".into(),
                },
            ])
            .unwrap();
        let nas_id = match store.find_identity_id("Lab NAS").unwrap() {
            network_manager_db::IdentityLookup::Found(id) => id,
            other => panic!("expected identity, got {other:?}"),
        };
        store
            .set_tracked_state_by_id(&nas_id, TrackedState::Untracked, Some("Lab NAS"), None)
            .unwrap();

        let detail = SqliteRepository::new(&path).selected_device_detail(Some(&nas_id));

        assert_eq!(detail.identity.id, nas_id);
        assert_eq!(detail.identity.label, "Lab NAS");
        assert_eq!(detail.device_list.len(), 2);
    }

    #[test]
    fn inferred_device_type_uses_names_services_and_gateway_ips() {
        assert_eq!(
            infer_device_type(&["Fabian-MacBook-Pro".into()]),
            Some("Mac".into())
        );
        assert_eq!(
            infer_device_type(&["local:_ipp._tcp:Office LaserJet".into()]),
            Some("Printer".into())
        );
        assert_eq!(infer_ip_device_type("192.168.178.1"), "Router / Gateway");
        assert_eq!(
            infer_gateway_type("192-168-178-1"),
            Some("Router / Gateway".into())
        );
    }

    #[test]
    fn tailscale_service_states_do_not_overstate_availability() {
        assert_eq!(
            parse_tailscale_service_state("Running"),
            AvailabilityState::Online
        );
        assert_eq!(
            parse_tailscale_service_state("Stopped"),
            AvailabilityState::Offline
        );
        assert_eq!(
            parse_tailscale_service_state("NeedsLogin"),
            AvailabilityState::Unknown
        );
        assert_eq!(
            parse_tailscale_service_state("Starting"),
            AvailabilityState::Unknown
        );
    }
}
