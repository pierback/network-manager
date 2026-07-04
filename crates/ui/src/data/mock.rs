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

    fn detail_device_list(&self) -> Vec<DeviceIdentityVm> {
        self.tracked_rows()
            .into_iter()
            .map(|row| DeviceIdentityVm {
                id: row.id,
                label: row.label,
                alias: row.alias,
                category: row.category,
                tracked_state: TrackedState::Tracked,
                availability: row.overall,
                ssh_username: None,
                endpoint_preference: EndpointPreference::Auto,
            })
            .collect()
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
                local_ip_address: "192.168.1.10".into(),
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
            filters: vec![
                "All sources".into(),
                "LAN".into(),
                "Tailscale".into(),
                "SSH capable".into(),
                "Untracked".into(),
            ],
            possible_match: Some(
                "MacBook-Pro.local and office-mbp.tailnet.ts.net share host evidence; auto-merged with high confidence."
                    .into(),
            ),
            rows: vec![
                DiscoveryRowVm {
                    id: "office-mbp".into(),
                    identity_id: Some("office-mbp".into()),
                    display_name: "Office MacBook".into(),
                    hostname: "office-mbp.local".into(),
                    ip_address: "192.168.1.10".into(),
                    source: "Merged identity".into(),
                    sources: vec!["LAN".into(), "mDNS".into(), "Tailscale".into()],
                    category: "Mac".into(),
                    tracked_state: TrackedState::Tracked,
                    availability: AvailabilityState::Online,
                    ssh_capable: true,
                    last_seen: "30s ago".into(),
                },
                DiscoveryRowVm {
                    id: "nas-main".into(),
                    identity_id: Some("nas-main".into()),
                    display_name: "Synology NAS".into(),
                    hostname: "nas.tailnet.ts.net".into(),
                    ip_address: "100.88.2.12".into(),
                    source: "Tailscale".into(),
                    sources: vec!["Tailscale".into(), "DNS".into()],
                    category: "Storage".into(),
                    tracked_state: TrackedState::Tracked,
                    availability: AvailabilityState::Online,
                    ssh_capable: true,
                    last_seen: "1m ago".into(),
                },
                DiscoveryRowVm {
                    id: "router".into(),
                    identity_id: Some("router".into()),
                    display_name: "Home Router".into(),
                    hostname: "router.local".into(),
                    ip_address: "192.168.1.1".into(),
                    source: "ARP".into(),
                    sources: vec!["LAN".into(), "ARP".into()],
                    category: "Router".into(),
                    tracked_state: TrackedState::Untracked,
                    availability: AvailabilityState::Online,
                    ssh_capable: false,
                    last_seen: "2m ago".into(),
                },
                DiscoveryRowVm {
                    id: "printer-hp".into(),
                    identity_id: Some("printer-hp".into()),
                    display_name: "HP LaserJet".into(),
                    hostname: "hp-laserjet.local".into(),
                    ip_address: "192.168.1.42".into(),
                    source: "mDNS".into(),
                    sources: vec!["LAN".into(), "mDNS".into()],
                    category: "Printer".into(),
                    tracked_state: TrackedState::Tracked,
                    availability: AvailabilityState::Unknown,
                    ssh_capable: false,
                    last_seen: "18m ago".into(),
                },
                DiscoveryRowVm {
                    id: "guest-phone".into(),
                    identity_id: None,
                    display_name: "Guest iPhone".into(),
                    hostname: "—".into(),
                    ip_address: "192.168.1.77".into(),
                    source: "ARP".into(),
                    sources: vec!["LAN".into()],
                    category: "Phone".into(),
                    tracked_state: TrackedState::Untracked,
                    availability: AvailabilityState::Unknown,
                    ssh_capable: false,
                    last_seen: "31m ago".into(),
                },
            ],
        }
    }

    fn selected_device_detail(&self, selected_identity_id: Option<&str>) -> DeviceDetailVm {
        let device_list = self.detail_device_list();
        if let Some(selected) = selected_identity_id
            .filter(|identity_id| *identity_id != "nas-main")
            .and_then(|identity_id| {
                device_list
                    .iter()
                    .find(|device| device.id == identity_id)
                    .cloned()
            })
        {
            return DeviceDetailVm {
                evidence: vec![format!("Mock identity: {}", selected.id)],
                identity: selected,
                device_list,
                endpoints: Vec::new(),
                preferred_target: None,
            };
        }

        let identity = DeviceIdentityVm {
            id: "nas-main".into(),
            label: "Synology NAS".into(),
            alias: "nas-main".into(),
            category: "Storage".into(),
            tracked_state: TrackedState::Tracked,
            availability: AvailabilityState::Online,
            ssh_username: Some("admin".into()),
            endpoint_preference: EndpointPreference::Auto,
        };
        DeviceDetailVm {
            identity,
            device_list,
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
                    hostname: Some("nas.local".into()),
                    port: Some(22),
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
                    hostname: None,
                    port: Some(22),
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
                    hostname: Some("nas.tailnet.ts.net".into()),
                    port: Some(22),
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
                    hostname: None,
                    port: Some(22),
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
            daemon: DaemonStatusVm {
                state: AvailabilityState::Online,
                source: "mock daemon".into(),
                tailscale_service: AvailabilityState::Online,
                local_ip_address: "192.168.1.10".into(),
                last_scan: "12s ago".into(),
                stale: false,
            },
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
        let detail = MockRepository::new().selected_device_detail(None);
        assert_eq!(detail.device_list.len(), 5);
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
