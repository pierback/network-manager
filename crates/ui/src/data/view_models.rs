use network_manager_core::{AvailabilityState, EndpointKind, EndpointPreference, TrackedState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionStatus {
    pub message: String,
    pub detail: Option<String>,
    pub is_error: bool,
    pub is_pending: bool,
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
    pub filters: Vec<String>,
    pub possible_match: Option<String>,
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

impl EndpointGroup {
    pub fn label(self) -> &'static str {
        match self {
            EndpointGroup::Lan => "LAN",
            EndpointGroup::Tailscale => "Tailscale",
            EndpointGroup::Other => "Other",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SshTargetVm {
    pub destination: String,
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
    pub discovery_interval: String,
    pub battery_mode: bool,
    pub tailscale_enabled: bool,
    pub tailscale_status: AvailabilityState,
    pub ssh_config_export: bool,
    pub debug_logging: bool,
}
