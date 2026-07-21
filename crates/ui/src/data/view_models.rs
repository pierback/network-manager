use network_manager_core::{AvailabilityState, EndpointKind, EndpointPreference, TrackedState};
use std::net::IpAddr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionStatus {
    pub message: String,
    pub detail: Option<String>,
    phase: ActionPhase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActionPhase {
    Pending,
    Succeeded,
    Failed,
}

impl ActionStatus {
    pub fn pending(message: impl Into<String>) -> Self {
        Self::new(message, None, ActionPhase::Pending)
    }

    pub fn succeeded(message: impl Into<String>, detail: Option<String>) -> Self {
        Self::new(message, detail, ActionPhase::Succeeded)
    }

    pub fn failed(message: impl Into<String>, detail: Option<String>) -> Self {
        Self::new(message, detail, ActionPhase::Failed)
    }

    pub fn is_pending(&self) -> bool {
        self.phase == ActionPhase::Pending
    }

    pub fn is_error(&self) -> bool {
        self.phase == ActionPhase::Failed
    }

    fn new(message: impl Into<String>, detail: Option<String>, phase: ActionPhase) -> Self {
        Self {
            message: message.into(),
            detail,
            phase,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonStatusVm {
    pub state: AvailabilityState,
    pub source: String,
    pub tailscale_service: AvailabilityState,
    pub local_ip_address: String,
    pub last_scan: String,
    pub stale: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DashboardVm {
    pub daemon: DaemonStatusVm,
    pub tracked: Vec<TrackedDeviceRowVm>,
    pub online_count: usize,
    pub tailscale_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackedDeviceRowVm {
    pub id: String,
    pub label: String,
    pub alias: String,
    pub category: String,
    pub overall: AvailabilityState,
    pub lan: AvailabilityState,
    pub tailscale: AvailabilityState,
    pub ssh: AvailabilityState,
    pub preferred_target: String,
    pub target_reason: String,
    pub last_seen: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryVm {
    pub rows: Vec<DiscoveryRowVm>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryRowVm {
    pub id: String,
    pub identity_id: Option<String>,
    pub display_name: String,
    pub hostname: String,
    pub ip_address: String,
    pub source: String,
    pub sources: Vec<String>,
    pub category: String,
    pub tracked_state: TrackedState,
    pub availability: AvailabilityState,
    pub ssh_capable: bool,
    pub last_seen: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceDetailVm {
    pub identity: DeviceIdentityVm,
    pub device_list: Vec<DeviceIdentityVm>,
    pub endpoints: Vec<EndpointVm>,
    pub preferred_target: Option<SshTargetVm>,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceIdentityVm {
    pub id: String,
    pub label: String,
    pub alias: String,
    pub category: String,
    pub tracked_state: TrackedState,
    pub availability: AvailabilityState,
    pub ssh_username: Option<String>,
    pub endpoint_preference: EndpointPreference,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndpointVm {
    pub id: String,
    pub group: EndpointGroup,
    pub kind: EndpointKind,
    pub address: String,
    pub hostname: Option<String>,
    pub port: Option<u16>,
    pub reachability: AvailabilityState,
    pub ssh_capability: AvailabilityState,
    pub last_checked: String,
    pub preferred: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndpointGroup {
    Lan,
    Tailscale,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndpointSectionVm {
    pub title: &'static str,
    pub description: &'static str,
    pub primary: String,
    pub host: String,
    pub ip: String,
    pub port: String,
    pub last_checked: String,
    pub reachability: AvailabilityState,
}

impl DeviceDetailVm {
    pub fn endpoint_sections(&self) -> [EndpointSectionVm; 3] {
        [
            EndpointSectionVm::from_endpoints(
                "LAN Endpoints",
                "Network Proximity · SSH capable",
                self.endpoints
                    .iter()
                    .filter(|endpoint| endpoint.group == EndpointGroup::Lan),
            ),
            EndpointSectionVm::from_endpoints(
                "Tailscale Endpoints",
                "Tailscale Presence · SSH capable",
                self.endpoints
                    .iter()
                    .filter(|endpoint| endpoint.group == EndpointGroup::Tailscale),
            ),
            EndpointSectionVm::from_endpoints(
                "Observed Names",
                "Identity Evidence · discovery",
                self.endpoints
                    .iter()
                    .filter(|endpoint| !is_ip_address(&endpoint.address)),
            ),
        ]
    }
}

impl EndpointSectionVm {
    fn from_endpoints<'a>(
        title: &'static str,
        description: &'static str,
        endpoints: impl IntoIterator<Item = &'a EndpointVm>,
    ) -> Self {
        let endpoints = endpoints.into_iter().collect::<Vec<_>>();
        let host = endpoint_host(&endpoints);
        let ips = endpoint_ips(&endpoints);
        let primary = host
            .clone()
            .or_else(|| ips.first().cloned())
            .unwrap_or_else(|| "—".into());
        let host = host.unwrap_or_else(|| "—".into());
        let ip = if ips.is_empty() {
            "—".into()
        } else {
            ips.join(", ")
        };
        let port = endpoints
            .iter()
            .find_map(|endpoint| endpoint.port)
            .map(|port| port.to_string())
            .unwrap_or_else(|| "—".into());
        let last_checked = endpoints
            .iter()
            .map(|endpoint| endpoint.last_checked.as_str())
            .find(|value| *value != "never")
            .or_else(|| {
                endpoints
                    .first()
                    .map(|endpoint| endpoint.last_checked.as_str())
            })
            .unwrap_or("—")
            .to_string();
        let reachability =
            AvailabilityState::aggregate(endpoints.iter().map(|endpoint| endpoint.reachability));

        Self {
            title,
            description,
            primary,
            host,
            ip,
            port,
            last_checked,
            reachability,
        }
    }
}

fn endpoint_host(endpoints: &[&EndpointVm]) -> Option<String> {
    endpoints
        .iter()
        .find_map(|endpoint| endpoint.hostname.as_deref())
        .or_else(|| {
            endpoints
                .iter()
                .map(|endpoint| endpoint.address.as_str())
                .find(|address| !is_ip_address(address))
        })
        .map(ToString::to_string)
}

fn endpoint_ips(endpoints: &[&EndpointVm]) -> Vec<String> {
    let mut ips = Vec::new();
    for endpoint in endpoints {
        if is_ip_address(&endpoint.address) && !ips.contains(&endpoint.address) {
            ips.push(endpoint.address.clone());
        }
    }
    ips
}

fn is_ip_address(value: &str) -> bool {
    value.parse::<IpAddr>().is_ok()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshTargetVm {
    pub destination: String,
    pub command: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickAccessVm {
    pub rows: Vec<TrackedDeviceRowVm>,
    pub last_scan: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsVm {
    pub daemon: DaemonStatusVm,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn endpoint(
        id: &str,
        group: EndpointGroup,
        kind: EndpointKind,
        address: &str,
        hostname: Option<&str>,
        reachability: AvailabilityState,
        last_checked: &str,
    ) -> EndpointVm {
        EndpointVm {
            id: id.into(),
            group,
            kind,
            address: address.into(),
            hostname: hostname.map(str::to_string),
            port: Some(22),
            reachability,
            ssh_capability: AvailabilityState::Unknown,
            last_checked: last_checked.into(),
            preferred: false,
        }
    }

    fn detail(endpoints: Vec<EndpointVm>) -> DeviceDetailVm {
        DeviceDetailVm {
            identity: DeviceIdentityVm {
                id: "nas".into(),
                label: "NAS".into(),
                alias: "nas".into(),
                category: "Storage".into(),
                tracked_state: TrackedState::Tracked,
                availability: AvailabilityState::Online,
                ssh_username: None,
                endpoint_preference: EndpointPreference::Auto,
            },
            device_list: Vec::new(),
            endpoints,
            preferred_target: None,
            evidence: Vec::new(),
        }
    }

    #[test]
    fn endpoint_sections_project_grouped_card_values() {
        let detail = detail(vec![
            endpoint(
                "lan-name",
                EndpointGroup::Lan,
                EndpointKind::LanDns,
                "nas.local",
                Some("nas.local"),
                AvailabilityState::Offline,
                "never",
            ),
            endpoint(
                "lan-ip",
                EndpointGroup::Lan,
                EndpointKind::LanIp,
                "192.168.1.10",
                None,
                AvailabilityState::Online,
                "12s ago",
            ),
            endpoint(
                "lan-ip-duplicate",
                EndpointGroup::Lan,
                EndpointKind::LanIp,
                "192.168.1.10",
                None,
                AvailabilityState::Unknown,
                "never",
            ),
            endpoint(
                "tailscale-name",
                EndpointGroup::Tailscale,
                EndpointKind::TailscaleDns,
                "nas.tailnet.ts.net",
                Some("nas.tailnet.ts.net"),
                AvailabilityState::Offline,
                "5s ago",
            ),
        ]);

        let [lan, tailscale, observed] = detail.endpoint_sections();

        assert_eq!(lan.title, "LAN Endpoints");
        assert_eq!(lan.primary, "nas.local");
        assert_eq!(lan.host, "nas.local");
        assert_eq!(lan.ip, "192.168.1.10");
        assert_eq!(lan.port, "22");
        assert_eq!(lan.last_checked, "12s ago");
        assert_eq!(lan.reachability, AvailabilityState::Online);

        assert_eq!(tailscale.primary, "nas.tailnet.ts.net");
        assert_eq!(tailscale.ip, "—");
        assert_eq!(tailscale.reachability, AvailabilityState::Offline);

        assert_eq!(observed.primary, "nas.local");
        assert_eq!(observed.ip, "—");
        assert_eq!(observed.last_checked, "5s ago");
        assert_eq!(observed.reachability, AvailabilityState::Offline);
    }

    #[test]
    fn endpoint_section_falls_back_to_ip_without_a_hostname() {
        let detail = detail(vec![endpoint(
            "lan-ip",
            EndpointGroup::Lan,
            EndpointKind::LanIp,
            "192.168.1.10",
            None,
            AvailabilityState::Unknown,
            "never",
        )]);

        let [lan, _, _] = detail.endpoint_sections();

        assert_eq!(lan.primary, "192.168.1.10");
        assert_eq!(lan.host, "—");
        assert_eq!(lan.ip, "192.168.1.10");
        assert_eq!(lan.last_checked, "never");
    }
}
