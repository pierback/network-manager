use std::{
    collections::HashMap,
    net::Ipv4Addr,
    path::{Path, PathBuf},
    process::Command,
};

use network_manager_core::{
    resolve_ssh_target, AvailabilityState, DeviceIdentity, EndpointKind, EndpointPreference,
    NetworkEndpoint, SshTarget, TrackedState,
};
use network_manager_db::{DeviceDetails, DiscoveredDeviceRecord, SqliteStore};

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
        let Ok(endpoints_by_identity) = endpoints_by_identity(&store) else {
            return DashboardVm {
                online_count: 0,
                tailscale_count: 0,
                tracked: Vec::new(),
                daemon,
            };
        };
        let tracked = store
            .list_device_identities()
            .unwrap_or_default()
            .into_iter()
            .filter(|record| record.identity.tracked_state == TrackedState::Tracked)
            .map(|record| {
                let endpoints = endpoints_by_identity
                    .get(&record.identity.id)
                    .cloned()
                    .unwrap_or_default();
                tracked_row_from_parts(record.identity, endpoints)
            })
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
            return DiscoveryVm { rows: Vec::new() };
        };
        let identities_by_id = store
            .list_device_identities()
            .unwrap_or_default()
            .into_iter()
            .map(|record| (record.identity.id.clone(), record.identity))
            .collect::<HashMap<_, _>>();
        let endpoints_by_identity = endpoints_by_identity(&store).ok();
        let rows = collapse_discovery_rows(
            store
                .list_discovered_devices()
                .unwrap_or_default()
                .into_iter()
                .filter(|record| !is_noisy_discovery(record))
                .map(|record| {
                    let details = discovery_details(
                        &record,
                        &identities_by_id,
                        endpoints_by_identity.as_ref(),
                    );
                    discovery_row(record, details)
                })
                .collect(),
        );
        DiscoveryVm { rows }
    }

    fn selected_device_detail(&self, selected_identity_id: Option<&str>) -> DeviceDetailVm {
        let Ok(store) = self.store() else {
            return empty_detail("sqlite unavailable", Vec::new());
        };
        let identities = store.list_device_identities().unwrap_or_default();
        let Ok(endpoints_by_identity) = endpoints_by_identity(&store) else {
            return empty_detail("Device details unavailable", Vec::new());
        };
        let device_list = identities
            .iter()
            .map(|record| {
                let endpoints = endpoints_by_identity
                    .get(&record.identity.id)
                    .cloned()
                    .unwrap_or_default();
                device_identity_vm_with_endpoints(&record.identity, &endpoints)
            })
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
        let endpoints = endpoints_by_identity
            .get(&selected.identity.id)
            .cloned()
            .unwrap_or_default();
        device_detail(
            DeviceDetails {
                identity: selected.identity.clone(),
                endpoints,
            },
            device_list,
        )
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
        SettingsVm { daemon }
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
        local_ip_address: local_ip_address_label(),
        last_scan: status.updated_at.unwrap_or_else(|| "never".into()),
        stale: status.stale,
    }
}

fn daemon_status(source: &str) -> DaemonStatusVm {
    DaemonStatusVm {
        state: AvailabilityState::Unknown,
        source: source.into(),
        tailscale_service: AvailabilityState::Unknown,
        local_ip_address: local_ip_address_label(),
        last_scan: "never".into(),
        stale: true,
    }
}

fn local_ip_address_label() -> String {
    current_mac_lan_ip().unwrap_or_else(|| "No LAN IP".into())
}

fn current_mac_lan_ip() -> Option<String> {
    let output = Command::new("ifconfig").arg("-a").output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8(output.stdout).ok()?;
    parse_local_lan_ip(&stdout)
}

fn parse_local_lan_ip(ifconfig_output: &str) -> Option<String> {
    ifconfig_output
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            (parts.next()? == "inet")
                .then(|| parts.next())
                .flatten()
                .and_then(|addr| addr.parse::<Ipv4Addr>().ok())
        })
        .find(is_lan_ipv4)
        .map(|addr| addr.to_string())
}

fn is_lan_ipv4(addr: &Ipv4Addr) -> bool {
    let [first, second, _, _] = addr.octets();
    !(first == 0
        || first == 127
        || first == 169 && second == 254
        || (224..=239).contains(&first)
        || first == 100 && (64..=127).contains(&second))
}

fn empty_dashboard(source: &str) -> DashboardVm {
    DashboardVm {
        daemon: daemon_status(source),
        tracked: Vec::new(),
        online_count: 0,
        tailscale_count: 0,
    }
}

fn endpoints_by_identity(
    store: &SqliteStore,
) -> anyhow::Result<HashMap<String, Vec<NetworkEndpoint>>> {
    let mut endpoints_by_identity: HashMap<String, Vec<NetworkEndpoint>> = HashMap::new();
    for endpoint in store.list_endpoints_for_probe(false)? {
        endpoints_by_identity
            .entry(endpoint.identity_id.clone())
            .or_default()
            .push(endpoint);
    }
    Ok(endpoints_by_identity)
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
        .map(SshTarget::destination)
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

fn discovery_details(
    record: &DiscoveredDeviceRecord,
    identities_by_id: &HashMap<String, DeviceIdentity>,
    endpoints_by_identity: Option<&HashMap<String, Vec<NetworkEndpoint>>>,
) -> Option<DeviceDetails> {
    let identity_id = record.identity_id.as_ref()?;
    let endpoints_by_identity = endpoints_by_identity?;
    Some(DeviceDetails {
        identity: identities_by_id.get(identity_id)?.clone(),
        endpoints: endpoints_by_identity
            .get(identity_id)
            .cloned()
            .unwrap_or_default(),
    })
}

fn discovery_row(record: DiscoveredDeviceRecord, details: Option<DeviceDetails>) -> DiscoveryRowVm {
    let display_name = record
        .device
        .display_name
        .clone()
        .unwrap_or_else(|| record.device.source_device_id.clone());
    let (tracked_state, category, availability, sources, hostname, ip_address, ssh_capable) =
        details
            .as_ref()
            .map(|details| {
                (
                    details.identity.tracked_state,
                    details.identity.category.clone().unwrap_or_else(|| {
                        infer_device_type_for_discovery(&record, Some(details), &display_name)
                    }),
                    discovery_availability(&record, &details.endpoints),
                    discovery_sources(&record, &details.endpoints),
                    endpoint_hostname(&details.endpoints, &record.device.source_device_id),
                    endpoint_ip_address(&details.endpoints, &record.device.source_device_id),
                    discovery_ssh_capable(&record, &details.endpoints),
                )
            })
            .unwrap_or_else(|| {
                (
                    TrackedState::Untracked,
                    infer_device_type_for_discovery(&record, None, &display_name),
                    discovery_availability(&record, &[]),
                    vec![source_label(&record.device.source).into()],
                    record.device.source_device_id.clone(),
                    ip_like_or_dash(&record.device.source_device_id),
                    discovery_ssh_capable(&record, &[]),
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
        ssh_capable,
        last_seen: record.device.last_seen_at,
    }
}

fn collapse_discovery_rows(rows: Vec<DiscoveryRowVm>) -> Vec<DiscoveryRowVm> {
    DiscoveryProjection::collapse(rows)
}

struct DiscoveryProjection {
    rows: Vec<DiscoveryRowVm>,
    row_indexes_by_key: HashMap<String, usize>,
}

impl DiscoveryProjection {
    fn collapse(rows: Vec<DiscoveryRowVm>) -> Vec<DiscoveryRowVm> {
        let mut projection = Self {
            rows: Vec::with_capacity(rows.len()),
            row_indexes_by_key: HashMap::with_capacity(rows.len()),
        };
        for row in rows {
            projection.push(row);
        }
        projection.rows
    }

    fn push(&mut self, row: DiscoveryRowVm) {
        let key = canonical_discovery_key(&row);
        if let Some(index) = self.row_indexes_by_key.get(&key).copied() {
            let existing = self
                .rows
                .get_mut(index)
                .expect("discovery projection key index points at an existing row");
            merge_discovery_row(existing, row);
        } else {
            self.row_indexes_by_key.insert(key, self.rows.len());
            self.rows.push(row);
        }
    }
}

fn merge_discovery_row(existing: &mut DiscoveryRowVm, incoming: DiscoveryRowVm) {
    for source in incoming.sources {
        if !existing.sources.iter().any(|known| known == &source) {
            existing.sources.push(source);
        }
    }
    existing.availability =
        AvailabilityState::aggregate([existing.availability, incoming.availability]);
    existing.ssh_capable |= incoming.ssh_capable;
    existing.tracked_state = merge_tracked_state(existing.tracked_state, incoming.tracked_state);
    if existing.identity_id.is_none() {
        existing.identity_id = incoming.identity_id;
    }
    if existing.category == "Device" && incoming.category != "Device" {
        existing.category = incoming.category;
    }
    if existing.ip_address == "—" && incoming.ip_address != "—" {
        existing.ip_address = incoming.ip_address;
    }
    if existing.hostname == "—" || existing.hostname.starts_with("local:") {
        existing.hostname = incoming.hostname;
    }
    if should_prefer_display_name(&existing.display_name, &incoming.display_name) {
        existing.display_name = incoming.display_name;
    }
}

fn merge_tracked_state(left: TrackedState, right: TrackedState) -> TrackedState {
    match (left, right) {
        (TrackedState::Tracked, _) | (_, TrackedState::Tracked) => TrackedState::Tracked,
        (TrackedState::Ignored, _) | (_, TrackedState::Ignored) => TrackedState::Ignored,
        _ => TrackedState::Untracked,
    }
}

fn should_prefer_display_name(existing: &str, incoming: &str) -> bool {
    existing == "—"
        || (existing.contains('-') && !incoming.contains('-'))
        || (existing.parse::<std::net::IpAddr>().is_ok()
            && incoming.parse::<std::net::IpAddr>().is_err())
}

fn canonical_discovery_key(row: &DiscoveryRowVm) -> String {
    canonical_ip_like(&row.ip_address)
        .or_else(|| canonical_ip_like(&row.display_name))
        .or_else(|| canonical_ip_like(&row.hostname))
        .map(|ip| format!("ip:{ip}"))
        .or_else(|| row.identity_id.as_ref().map(|id| format!("identity:{id}")))
        .unwrap_or_else(|| {
            format!(
                "name:{}",
                row.display_name
                    .to_ascii_lowercase()
                    .chars()
                    .filter(|ch| ch.is_ascii_alphanumeric())
                    .collect::<String>()
            )
        })
}

fn canonical_ip_like(value: &str) -> Option<String> {
    let candidate = value.trim().replace('-', ".");
    candidate
        .parse::<std::net::IpAddr>()
        .ok()
        .map(|ip| ip.to_string())
}

fn discovery_availability(
    record: &DiscoveredDeviceRecord,
    endpoints: &[NetworkEndpoint],
) -> AvailabilityState {
    let aggregate = aggregate_reachability(endpoints, |_| true);
    if aggregate == AvailabilityState::Unknown && record.device.source == "mdns" {
        AvailabilityState::Online
    } else {
        aggregate
    }
}

fn discovery_ssh_capable(record: &DiscoveredDeviceRecord, endpoints: &[NetworkEndpoint]) -> bool {
    aggregate_ssh(endpoints) == AvailabilityState::Online
        || record.device.source_device_id.contains("_ssh._tcp")
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
        identity: device_identity_vm_with_endpoints(&identity, &endpoints),
        device_list,
        endpoints: endpoints
            .iter()
            .map(|endpoint| EndpointVm {
                id: endpoint.id.clone(),
                group: endpoint_group(endpoint.kind),
                kind: endpoint.kind,
                address: endpoint.address.clone(),
                hostname: endpoint.hostname.clone(),
                port: endpoint.port,
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
        preferred_target: target.map(ssh_target_vm),
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
            availability: AvailabilityState::Unknown,
            ssh_username: None,
            endpoint_preference: EndpointPreference::Auto,
        },
        device_list,
        endpoints: Vec::new(),
        preferred_target: None,
        evidence: Vec::new(),
    }
}

fn device_identity_vm_with_endpoints(
    identity: &DeviceIdentity,
    endpoints: &[NetworkEndpoint],
) -> DeviceIdentityVm {
    DeviceIdentityVm {
        id: identity.id.clone(),
        label: identity_label(identity),
        alias: identity
            .alias
            .clone()
            .unwrap_or_else(|| identity.id.clone()),
        category: identity.category.clone().unwrap_or_else(|| "Device".into()),
        tracked_state: identity.tracked_state,
        availability: aggregate_reachability(endpoints, |_| true),
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
            "Home Automation",
            ["homeassistant", "home assistant", "hass", "homebridge"].as_slice(),
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
    AvailabilityState::aggregate(
        endpoints
            .iter()
            .filter(|endpoint| predicate(endpoint.kind))
            .map(|endpoint| endpoint.reachability),
    )
}

fn aggregate_ssh(endpoints: &[NetworkEndpoint]) -> AvailabilityState {
    AvailabilityState::aggregate(endpoints.iter().map(|endpoint| endpoint.ssh_capability))
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

fn ssh_target_vm(target: SshTarget) -> SshTargetVm {
    SshTargetVm {
        destination: target.destination(),
        command: target.shell_command(),
        reason: format!("Selected {} endpoint", target.endpoint_kind.as_str()),
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

    fn discovery_row_fixture(
        id: &str,
        display_name: &str,
        hostname: &str,
        ip_address: &str,
        source: &str,
        tracked_state: TrackedState,
        availability: AvailabilityState,
    ) -> DiscoveryRowVm {
        DiscoveryRowVm {
            id: id.into(),
            identity_id: None,
            display_name: display_name.into(),
            hostname: hostname.into(),
            ip_address: ip_address.into(),
            source: source.into(),
            sources: vec![source.into()],
            category: "Device".into(),
            tracked_state,
            availability,
            ssh_capable: false,
            last_seen: "now".into(),
        }
    }

    #[test]
    fn ssh_target_vm_uses_core_command_args_for_nondefault_port() {
        let target = ssh_target_vm(SshTarget {
            endpoint_id: "endpoint".into(),
            host: "device.local".into(),
            port: 2222,
            username: Some("alice".into()),
            endpoint_kind: EndpointKind::LanDns,
        });

        assert_eq!(target.destination, "alice@device.local");
        assert_eq!(target.command, "ssh -p 2222 alice@device.local");
    }

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
    fn discovery_collapses_duplicate_identity_rows_and_ip_like_names() {
        let tempdir = tempfile::tempdir().unwrap();
        let path = tempdir.path().join("test.sqlite");
        let store = SqliteStore::open(&path).unwrap();
        store.migrate().unwrap();
        store
            .record_mdns_services(&[
                network_manager_db::MdnsServiceObservation {
                    source_device_id: "local.:_ssh._tcp.:NAS".to_string(),
                    service_name: "NAS".to_string(),
                    service_type: "_ssh._tcp".to_string(),
                    domain: "local".to_string(),
                    hostname: Some("nas.local".to_string()),
                    ip_addresses: Vec::new(),
                    port: Some(22),
                    raw_text: "NAS._ssh._tcp.local. can be reached at nas.local.:22".to_string(),
                },
                network_manager_db::MdnsServiceObservation {
                    source_device_id: "local.:_smb._tcp.:NAS".to_string(),
                    service_name: "NAS".to_string(),
                    service_type: "_smb._tcp".to_string(),
                    domain: "local".to_string(),
                    hostname: Some("nas.local".to_string()),
                    ip_addresses: Vec::new(),
                    port: Some(445),
                    raw_text: "NAS._smb._tcp.local. can be reached at nas.local.:445".to_string(),
                },
                network_manager_db::MdnsServiceObservation {
                    source_device_id: "local.:_smb._tcp.:192-168-178-1".to_string(),
                    service_name: "192-168-178-1".to_string(),
                    service_type: "_smb._tcp".to_string(),
                    domain: "local".to_string(),
                    hostname: None,
                    ip_addresses: Vec::new(),
                    port: None,
                    raw_text: "192-168-178-1._smb._tcp.local.".to_string(),
                },
            ])
            .unwrap();
        store
            .record_lan_devices(&[LanDeviceObservation {
                ip_address: "192.168.178.1".into(),
                hostname: None,
                mac_address: Some("08:b6:57:4e:e2:cf".into()),
                interface_name: Some("en0".into()),
                raw_text: "? (192.168.178.1) at 08:b6:57:4e:e2:cf on en0".into(),
            }])
            .unwrap();

        let discovery = SqliteRepository::new(&path).discovery();

        assert_eq!(
            discovery
                .rows
                .iter()
                .filter(|row| row.display_name == "NAS")
                .count(),
            1
        );
        assert_eq!(
            discovery
                .rows
                .iter()
                .filter(|row| row.display_name.contains("192.168.178.1")
                    || row.display_name.contains("192-168-178-1"))
                .count(),
            1
        );
    }

    #[test]
    fn discovery_projection_preserves_identity_details_from_batched_lookup() {
        let tempdir = tempfile::tempdir().unwrap();
        let path = tempdir.path().join("test.sqlite");
        let store = SqliteStore::open(&path).unwrap();
        store.migrate().unwrap();
        store
            .record_lan_devices(&[LanDeviceObservation {
                ip_address: "192.168.1.20".into(),
                hostname: Some("Lab NAS".into()),
                mac_address: Some("AA:BB:CC:00:11:33".into()),
                interface_name: Some("en0".into()),
                raw_text: "Lab NAS (192.168.1.20) at aa:bb:cc:00:11:33 on en0".into(),
            }])
            .unwrap();
        let identity_id = match store.find_identity_id("Lab NAS").unwrap() {
            network_manager_db::IdentityLookup::Found(id) => id,
            other => panic!("expected identity, got {other:?}"),
        };
        store
            .set_tracked_state_by_id(&identity_id, TrackedState::Tracked, Some("Lab NAS"), None)
            .unwrap();
        store
            .set_category_by_id(&identity_id, Some("NAS / Storage"))
            .unwrap();

        let discovery = SqliteRepository::new(&path).discovery();
        let row = discovery
            .rows
            .iter()
            .find(|row| row.identity_id.as_deref() == Some(identity_id.as_str()))
            .expect("tracked LAN identity appears in discovery");

        assert_eq!(row.tracked_state, TrackedState::Tracked);
        assert_eq!(row.category, "NAS / Storage");
        assert_eq!(row.hostname, "Lab NAS");
        assert_eq!(row.ip_address, "192.168.1.20");
        assert_eq!(row.sources, vec!["LAN".to_string(), "ARP".to_string()]);
    }

    #[test]
    fn discovery_projection_merges_duplicate_keys_without_reordering_rows() {
        let rows = vec![
            discovery_row_fixture(
                "router-lan",
                "192.168.1.1",
                "—",
                "192.168.1.1",
                "ARP",
                TrackedState::Untracked,
                AvailabilityState::Unknown,
            ),
            discovery_row_fixture(
                "nas-mdns",
                "NAS",
                "nas.local",
                "—",
                "mDNS",
                TrackedState::Untracked,
                AvailabilityState::Online,
            ),
            discovery_row_fixture(
                "router-mdns",
                "192-168-1-1",
                "—",
                "—",
                "mDNS",
                TrackedState::Tracked,
                AvailabilityState::Online,
            ),
        ];

        let collapsed = collapse_discovery_rows(rows);

        assert_eq!(collapsed.len(), 2);
        assert_eq!(collapsed[0].id, "router-lan");
        assert_eq!(
            collapsed[0].sources,
            vec!["ARP".to_string(), "mDNS".to_string()]
        );
        assert_eq!(collapsed[0].tracked_state, TrackedState::Tracked);
        assert_eq!(collapsed[0].availability, AvailabilityState::Online);
        assert_eq!(collapsed[1].id, "nas-mdns");
    }

    #[test]
    fn mdns_home_assistant_discovery_is_online_home_automation() {
        let tempdir = tempfile::tempdir().unwrap();
        let path = tempdir.path().join("test.sqlite");
        let store = SqliteStore::open(&path).unwrap();
        store.migrate().unwrap();
        store
            .record_mdns_services(&[network_manager_db::MdnsServiceObservation {
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

        let discovery = SqliteRepository::new(&path).discovery();

        assert_eq!(discovery.rows.len(), 1);
        assert_eq!(discovery.rows[0].category, "Home Automation");
        assert_eq!(discovery.rows[0].availability, AvailabilityState::Online);
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

    #[test]
    fn local_lan_ip_ignores_loopback_link_local_and_tailscale_cgnat() {
        let ifconfig_output = r#"
lo0: flags=8049<UP,LOOPBACK,RUNNING,MULTICAST> mtu 16384
        inet 127.0.0.1 netmask 0xff000000
utun4: flags=8051<UP,POINTOPOINT,RUNNING,MULTICAST> mtu 1280
        inet 100.99.88.77 --> 100.99.88.77 netmask 0xffffffff
en0: flags=8863<UP,BROADCAST,SMART,RUNNING,SIMPLEX,MULTICAST> mtu 1500
        inet 192.168.178.23 netmask 0xffffff00 broadcast 192.168.178.255
awdl0: flags=8943<UP,BROADCAST,RUNNING,PROMISC,SIMPLEX,MULTICAST> mtu 1500
        inet 169.254.20.7 netmask 0xffff0000 broadcast 169.254.255.255
"#;

        assert_eq!(
            parse_local_lan_ip(ifconfig_output),
            Some("192.168.178.23".into())
        );
    }

    #[test]
    fn local_lan_ip_returns_none_when_only_non_lan_addresses_exist() {
        let ifconfig_output = r#"
lo0: flags=8049<UP,LOOPBACK,RUNNING,MULTICAST> mtu 16384
        inet 127.0.0.1 netmask 0xff000000
utun4: flags=8051<UP,POINTOPOINT,RUNNING,MULTICAST> mtu 1280
        inet 100.77.66.55 --> 100.77.66.55 netmask 0xffffffff
awdl0: flags=8943<UP,BROADCAST,RUNNING,PROMISC,SIMPLEX,MULTICAST> mtu 1500
        inet 169.254.20.7 netmask 0xffff0000 broadcast 169.254.255.255
"#;

        assert_eq!(parse_local_lan_ip(ifconfig_output), None);
    }
}
