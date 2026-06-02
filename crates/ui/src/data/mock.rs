use network_manager_core::{AvailabilityState, EndpointKind, EndpointPreference, TrackedState};

use super::{
    DaemonStatusVm, DashboardVm, DeviceDetailVm, DeviceIdentityVm, DiscoveryRowVm, DiscoveryVm,
    EndpointGroup, EndpointVm, NetworkManagerRepository, QuickAccessVm, SettingsVm, SshTargetVm,
    TrackedDeviceRowVm,
};

#[derive(Debug, Clone, Default)]
pub struct MockRepository;

impl MockRepository {
    pub fn new() -> Self {
        Self
    }

    fn tracked_rows(&self) -> Vec<TrackedDeviceRowVm> {
        vec![
            TrackedDeviceRowVm {
                id: "office-mbp".into(),
                label: "Office MacBook".into(),
                alias: "office-macbook".into(),
                category: "Mac".into(),
                overall: AvailabilityState::Online,
                lan: AvailabilityState::Online,
                tailscale: AvailabilityState::Online,
                ssh: AvailabilityState::Online,
                preferred_target: "office-mbp.local".into(),
                target_reason: "LAN preferred because local reachability is proven".into(),
                last_seen: "30s ago".into(),
            },
            TrackedDeviceRowVm {
                id: "nas-main".into(),
                label: "Synology NAS".into(),
                alias: "nas-main".into(),
                category: "Storage".into(),
                overall: AvailabilityState::Online,
                lan: AvailabilityState::Unknown,
                tailscale: AvailabilityState::Online,
                ssh: AvailabilityState::Online,
                preferred_target: "nas.tailnet.ts.net".into(),
                target_reason: "Tailscale selected because LAN is stale".into(),
                last_seen: "1m ago".into(),
            },
            TrackedDeviceRowVm {
                id: "apple-tv".into(),
                label: "Living Room Apple TV".into(),
                alias: "apple-tv".into(),
                category: "Media".into(),
                overall: AvailabilityState::Online,
                lan: AvailabilityState::Online,
                tailscale: AvailabilityState::Unknown,
                ssh: AvailabilityState::Offline,
                preferred_target: "No SSH capability".into(),
                target_reason: "Reachable device, not an SSH target".into(),
                last_seen: "4m ago".into(),
            },
            TrackedDeviceRowVm {
                id: "printer-hp".into(),
                label: "HP LaserJet".into(),
                alias: "printer-hp".into(),
                category: "Printer".into(),
                overall: AvailabilityState::Unknown,
                lan: AvailabilityState::Unknown,
                tailscale: AvailabilityState::Offline,
                ssh: AvailabilityState::Offline,
                preferred_target: "No SSH capability".into(),
                target_reason: "Printer is tracked for visibility only".into(),
                last_seen: "18m ago".into(),
            },
            TrackedDeviceRowVm {
                id: "iphone".into(),
                label: "iPhone".into(),
                alias: "iphone".into(),
                category: "Phone".into(),
                overall: AvailabilityState::Unknown,
                lan: AvailabilityState::Unknown,
                tailscale: AvailabilityState::Online,
                ssh: AvailabilityState::Unknown,
                preferred_target: "iphone.tailnet.ts.net".into(),
                target_reason: "Tailscale is the only current endpoint".into(),
                last_seen: "42m ago".into(),
            },
        ]
    }
}

impl NetworkManagerRepository for MockRepository {
    fn dashboard(&self) -> DashboardVm {
        let tracked = self.tracked_rows();
        DashboardVm {
            daemon: DaemonStatusVm {
                state: AvailabilityState::Online,
                source: "mock daemon".into(),
                tailscale_service: AvailabilityState::Online,
                last_scan: "12s ago".into(),
                stale: false,
            },
            online_count: tracked
                .iter()
                .filter(|row| row.overall == AvailabilityState::Online)
                .count(),
            tailscale_count: tracked
                .iter()
                .filter(|row| row.tailscale == AvailabilityState::Online)
                .count(),
            tracked,
        }
    }

    fn discovery(&self) -> DiscoveryVm {
        DiscoveryVm {
            filters: vec!["All sources".into(), "Online".into(), "Untracked".into()],
            possible_match: Some(
                "MacBook-Pro.local and office-mbp.tailnet.ts.net share host evidence; auto-merged with high confidence."
                    .into(),
            ),
            rows: vec![
                DiscoveryRowVm {
                    id: "office-mbp".into(),
                    display_name: "Office MacBook".into(),
                    source: "Merged identity".into(),
                    sources: vec!["LAN".into(), "mDNS".into(), "Tailscale".into()],
                    category: "Mac".into(),
                    tracked_state: TrackedState::Tracked,
                    availability: AvailabilityState::Online,
                    last_seen: "30s ago".into(),
                },
                DiscoveryRowVm {
                    id: "nas-main".into(),
                    display_name: "Synology NAS".into(),
                    source: "Tailscale".into(),
                    sources: vec!["Tailscale".into(), "DNS".into()],
                    category: "Storage".into(),
                    tracked_state: TrackedState::Tracked,
                    availability: AvailabilityState::Online,
                    last_seen: "1m ago".into(),
                },
                DiscoveryRowVm {
                    id: "router".into(),
                    display_name: "Home Router".into(),
                    source: "ARP".into(),
                    sources: vec!["LAN".into(), "ARP".into()],
                    category: "Router".into(),
                    tracked_state: TrackedState::Untracked,
                    availability: AvailabilityState::Online,
                    last_seen: "2m ago".into(),
                },
                DiscoveryRowVm {
                    id: "printer-hp".into(),
                    display_name: "HP LaserJet".into(),
                    source: "mDNS".into(),
                    sources: vec!["LAN".into(), "mDNS".into()],
                    category: "Printer".into(),
                    tracked_state: TrackedState::Tracked,
                    availability: AvailabilityState::Unknown,
                    last_seen: "18m ago".into(),
                },
                DiscoveryRowVm {
                    id: "guest-phone".into(),
                    display_name: "Guest iPhone".into(),
                    source: "ARP".into(),
                    sources: vec!["LAN".into()],
                    category: "Phone".into(),
                    tracked_state: TrackedState::Untracked,
                    availability: AvailabilityState::Unknown,
                    last_seen: "31m ago".into(),
                },
            ],
        }
    }

    fn selected_device_detail(&self) -> DeviceDetailVm {
        DeviceDetailVm {
            identity: DeviceIdentityVm {
                id: "nas-main".into(),
                label: "Synology NAS".into(),
                alias: "nas-main".into(),
                category: "Storage".into(),
                tracked_state: TrackedState::Tracked,
                ssh_username: Some("admin".into()),
                endpoint_preference: EndpointPreference::Auto,
            },
            preferred_target: Some(SshTargetVm {
                destination: "admin@nas.tailnet.ts.net".into(),
                reason: "Using Tailscale because LAN reachability is stale.".into(),
            }),
            endpoints: vec![
                EndpointVm {
                    id: "nas-lan-dns".into(),
                    group: EndpointGroup::Lan,
                    kind: EndpointKind::LanDns,
                    address: "nas.local".into(),
                    reachability: AvailabilityState::Unknown,
                    ssh_capability: AvailabilityState::Online,
                    last_checked: "15m ago".into(),
                    preferred: false,
                },
                EndpointVm {
                    id: "nas-lan-ip".into(),
                    group: EndpointGroup::Lan,
                    kind: EndpointKind::LanIp,
                    address: "192.168.4.18".into(),
                    reachability: AvailabilityState::Unknown,
                    ssh_capability: AvailabilityState::Online,
                    last_checked: "15m ago".into(),
                    preferred: false,
                },
                EndpointVm {
                    id: "nas-ts-dns".into(),
                    group: EndpointGroup::Tailscale,
                    kind: EndpointKind::TailscaleDns,
                    address: "nas.tailnet.ts.net".into(),
                    reachability: AvailabilityState::Online,
                    ssh_capability: AvailabilityState::Online,
                    last_checked: "12s ago".into(),
                    preferred: true,
                },
                EndpointVm {
                    id: "nas-ts-ip".into(),
                    group: EndpointGroup::Tailscale,
                    kind: EndpointKind::TailscaleIp,
                    address: "100.88.2.12".into(),
                    reachability: AvailabilityState::Online,
                    ssh_capability: AvailabilityState::Online,
                    last_checked: "12s ago".into(),
                    preferred: false,
                },
            ],
            evidence: vec![
                "Tailscale node ID: node-8f12".into(),
                "mDNS hostname: nas.local".into(),
                "ARP MAC evidence confidence: 0.85".into(),
            ],
        }
    }

    fn quick_access(&self) -> QuickAccessVm {
        QuickAccessVm {
            rows: self.tracked_rows().into_iter().take(4).collect(),
            last_scan: "12s ago".into(),
        }
    }

    fn settings(&self) -> SettingsVm {
        SettingsVm {
            discovery_interval: "Every 5 minutes".into(),
            battery_mode: true,
            tailscale_enabled: true,
            tailscale_status: AvailabilityState::Online,
            ssh_config_export: false,
            debug_logging: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dashboard_mock_keeps_dashboard_tracked_only() {
        let repo = MockRepository::new();
        let dashboard = repo.dashboard();
        assert_eq!(dashboard.tracked.len(), 5);
        assert!(dashboard.tracked.iter().all(|row| !row.alias.is_empty()));
    }

    #[test]
    fn detail_mock_preserves_endpoint_grouping() {
        let detail = MockRepository::new().selected_device_detail();
        assert!(detail
            .endpoints
            .iter()
            .any(|endpoint| endpoint.group == EndpointGroup::Lan));
        assert!(detail
            .endpoints
            .iter()
            .any(|endpoint| endpoint.group == EndpointGroup::Tailscale));
        assert_eq!(
            detail
                .endpoints
                .iter()
                .filter(|endpoint| endpoint.preferred)
                .count(),
            1
        );
    }
}
