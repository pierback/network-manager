use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AvailabilityState {
    Online,
    Offline,
    Unknown,
}

impl AvailabilityState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Online => "online",
            Self::Offline => "offline",
            Self::Unknown => "unknown",
        }
    }

    pub fn aggregate(states: impl IntoIterator<Item = Self>) -> Self {
        states
            .into_iter()
            .max_by_key(|state| match state {
                Self::Online => 2,
                Self::Offline => 1,
                Self::Unknown => 0,
            })
            .unwrap_or(Self::Unknown)
    }
}

impl fmt::Display for AvailabilityState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for AvailabilityState {
    type Err = ParseDomainEnumError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "online" => Ok(Self::Online),
            "offline" => Ok(Self::Offline),
            "unknown" => Ok(Self::Unknown),
            other => Err(ParseDomainEnumError::new("AvailabilityState", other)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndpointKind {
    LanDns,
    Mdns,
    LanIp,
    TailscaleDns,
    TailscaleIp,
    Other,
}

impl EndpointKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LanDns => "lan_dns",
            Self::Mdns => "mdns",
            Self::LanIp => "lan_ip",
            Self::TailscaleDns => "tailscale_dns",
            Self::TailscaleIp => "tailscale_ip",
            Self::Other => "other",
        }
    }

    pub fn is_lan(self) -> bool {
        matches!(self, Self::LanDns | Self::Mdns | Self::LanIp)
    }

    pub fn is_tailscale(self) -> bool {
        matches!(self, Self::TailscaleDns | Self::TailscaleIp)
    }

    fn rank_within_group(self) -> u8 {
        match self {
            Self::LanDns => 0,
            Self::Mdns => 1,
            Self::LanIp => 2,
            Self::TailscaleDns => 0,
            Self::TailscaleIp => 1,
            Self::Other => 9,
        }
    }
}

impl fmt::Display for EndpointKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for EndpointKind {
    type Err = ParseDomainEnumError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "lan_dns" => Ok(Self::LanDns),
            "mdns" => Ok(Self::Mdns),
            "lan_ip" => Ok(Self::LanIp),
            "tailscale_dns" => Ok(Self::TailscaleDns),
            "tailscale_ip" => Ok(Self::TailscaleIp),
            "other" => Ok(Self::Other),
            other => Err(ParseDomainEnumError::new("EndpointKind", other)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndpointPreference {
    #[default]
    Auto,
    TailscaleFirst,
    #[serde(alias = "local_first")]
    LanFirst,
}

impl EndpointPreference {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::TailscaleFirst => "tailscale_first",
            Self::LanFirst => "lan_first",
        }
    }
}

impl fmt::Display for EndpointPreference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for EndpointPreference {
    type Err = ParseDomainEnumError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "auto" => Ok(Self::Auto),
            "local_first" | "lan_first" => Ok(Self::LanFirst),
            "tailscale_first" => Ok(Self::TailscaleFirst),
            other => Err(ParseDomainEnumError::new("EndpointPreference", other)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackedState {
    Untracked,
    Tracked,
    Ignored,
}

impl TrackedState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Untracked => "untracked",
            Self::Tracked => "tracked",
            Self::Ignored => "ignored",
        }
    }
}

impl fmt::Display for TrackedState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for TrackedState {
    type Err = ParseDomainEnumError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "untracked" => Ok(Self::Untracked),
            "tracked" => Ok(Self::Tracked),
            "ignored" => Ok(Self::Ignored),
            other => Err(ParseDomainEnumError::new("TrackedState", other)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceIdentity {
    pub id: String,
    pub stable_key: String,
    pub label: Option<String>,
    pub alias: Option<String>,
    pub tracked_state: TrackedState,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub ssh_username: Option<String>,
    pub ssh_port: Option<u16>,
    pub endpoint_preference: EndpointPreference,
    pub last_seen_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveredDevice {
    pub id: String,
    pub source: String,
    pub source_device_id: String,
    pub display_name: Option<String>,
    pub first_seen_at: String,
    pub last_seen_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveryObservation {
    pub id: Option<i64>,
    pub discovered_device_id: String,
    pub identity_id: Option<String>,
    pub source: String,
    pub observed_at: String,
    pub evidence_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkEndpoint {
    pub id: String,
    pub identity_id: String,
    pub kind: EndpointKind,
    pub address: String,
    pub port: Option<u16>,
    pub hostname: Option<String>,
    pub reachability: AvailabilityState,
    pub ssh_capability: AvailabilityState,
    pub last_seen_at: Option<String>,
    pub last_checked_at: Option<String>,
}

impl NetworkEndpoint {
    pub fn host_for_connection(&self) -> &str {
        self.hostname.as_deref().unwrap_or(&self.address)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SshTarget {
    pub endpoint_id: String,
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub endpoint_kind: EndpointKind,
}

impl SshTarget {
    pub fn destination(&self) -> String {
        match &self.username {
            Some(username) if !username.is_empty() => format!("{username}@{}", self.host),
            _ => self.host.clone(),
        }
    }

    pub fn command_args(&self) -> Vec<String> {
        let mut args = Vec::new();
        if self.port != 22 {
            args.push("-p".to_string());
            args.push(self.port.to_string());
        }
        args.push(self.destination());
        args
    }

    pub fn shell_command(&self) -> String {
        format_shell_command("ssh", &self.command_args())
    }
}

pub fn format_shell_command(program: &str, args: &[String]) -> String {
    std::iter::once(program)
        .chain(args.iter().map(String::as_str))
        .map(shell_word)
        .collect::<Vec<_>>()
        .join(" ")
}

fn shell_word(value: &str) -> String {
    if !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_@%+=:,./-".contains(&byte))
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\"'\"'"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("invalid {kind}: {value}")]
pub struct ParseDomainEnumError {
    kind: &'static str,
    value: String,
}

impl ParseDomainEnumError {
    fn new(kind: &'static str, value: impl Into<String>) -> Self {
        Self {
            kind,
            value: value.into(),
        }
    }
}

pub fn resolve_ssh_target(
    endpoints: &[NetworkEndpoint],
    preference: EndpointPreference,
    username: Option<&str>,
    ssh_port: Option<u16>,
) -> Option<SshTarget> {
    let candidates: Vec<&NetworkEndpoint> = endpoints
        .iter()
        .filter(|endpoint| endpoint.reachability != AvailabilityState::Offline)
        .filter(|endpoint| endpoint.ssh_capability != AvailabilityState::Offline)
        .collect();

    if candidates.is_empty() {
        return None;
    }

    let effective_preference = match preference {
        EndpointPreference::Auto => {
            if candidates.iter().any(|endpoint| {
                endpoint.kind.is_lan() && endpoint.reachability == AvailabilityState::Online
            }) {
                EndpointPreference::LanFirst
            } else {
                EndpointPreference::TailscaleFirst
            }
        }
        other => other,
    };

    let endpoint = candidates
        .into_iter()
        .min_by_key(|endpoint| ssh_score(endpoint, effective_preference))?;

    Some(SshTarget {
        endpoint_id: endpoint.id.clone(),
        host: endpoint.host_for_connection().to_string(),
        port: ssh_port.or(endpoint.port).unwrap_or(22),
        username: username.map(str::to_string),
        endpoint_kind: endpoint.kind,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct SshScore {
    proven: u8,
    transport: u8,
    reachability: u8,
    ssh_capability: u8,
    endpoint_kind: u8,
}

fn ssh_score(endpoint: &NetworkEndpoint, preference: EndpointPreference) -> SshScore {
    let proven_rank = u8::from(
        endpoint.reachability != AvailabilityState::Online
            || endpoint.ssh_capability != AvailabilityState::Online,
    );
    let group_rank = match preference {
        EndpointPreference::Auto | EndpointPreference::LanFirst => {
            if endpoint.kind.is_lan() {
                0
            } else if endpoint.kind.is_tailscale() {
                1
            } else {
                2
            }
        }
        EndpointPreference::TailscaleFirst => {
            if endpoint.kind.is_tailscale() {
                0
            } else if endpoint.kind.is_lan() {
                1
            } else {
                2
            }
        }
    };

    let reachability_rank = match endpoint.reachability {
        AvailabilityState::Online => 0,
        AvailabilityState::Unknown => 1,
        AvailabilityState::Offline => 9,
    };

    let ssh_rank = match endpoint.ssh_capability {
        AvailabilityState::Online => 0,
        AvailabilityState::Unknown => 1,
        AvailabilityState::Offline => 9,
    };

    SshScore {
        proven: proven_rank,
        transport: group_rank,
        reachability: reachability_rank,
        ssh_capability: ssh_rank,
        endpoint_kind: endpoint.kind.rank_within_group(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn endpoint(
        id: &str,
        kind: EndpointKind,
        host: &str,
        reachability: AvailabilityState,
    ) -> NetworkEndpoint {
        NetworkEndpoint {
            id: id.to_string(),
            identity_id: "identity-1".to_string(),
            kind,
            address: host.to_string(),
            port: None,
            hostname: if matches!(
                kind,
                EndpointKind::LanDns | EndpointKind::Mdns | EndpointKind::TailscaleDns
            ) {
                Some(host.to_string())
            } else {
                None
            },
            reachability,
            ssh_capability: AvailabilityState::Online,
            last_seen_at: None,
            last_checked_at: None,
        }
    }

    #[test]
    fn availability_aggregation_prefers_online_then_offline() {
        assert_eq!(
            AvailabilityState::aggregate([
                AvailabilityState::Unknown,
                AvailabilityState::Offline,
                AvailabilityState::Online,
            ]),
            AvailabilityState::Online
        );
        assert_eq!(
            AvailabilityState::aggregate([AvailabilityState::Unknown, AvailabilityState::Offline,]),
            AvailabilityState::Offline
        );
        assert_eq!(
            AvailabilityState::aggregate([AvailabilityState::Unknown]),
            AvailabilityState::Unknown
        );
    }

    #[test]
    fn auto_prefers_lan_name_when_lan_is_online() {
        let endpoints = vec![
            endpoint(
                "ts",
                EndpointKind::TailscaleDns,
                "mac.tailnet.ts.net",
                AvailabilityState::Online,
            ),
            endpoint(
                "lan",
                EndpointKind::LanDns,
                "macbook.local",
                AvailabilityState::Online,
            ),
            endpoint(
                "ip",
                EndpointKind::LanIp,
                "192.168.1.25",
                AvailabilityState::Online,
            ),
        ];

        let target =
            resolve_ssh_target(&endpoints, EndpointPreference::Auto, Some("franz"), None).unwrap();

        assert_eq!(target.endpoint_id, "lan");
        assert_eq!(target.destination(), "franz@macbook.local");
    }

    #[test]
    fn auto_falls_back_to_tailscale_when_lan_is_not_online() {
        let endpoints = vec![
            endpoint(
                "lan",
                EndpointKind::LanDns,
                "macbook.local",
                AvailabilityState::Offline,
            ),
            endpoint(
                "ts",
                EndpointKind::TailscaleDns,
                "mac.tailnet.ts.net",
                AvailabilityState::Online,
            ),
        ];

        let target = resolve_ssh_target(&endpoints, EndpointPreference::Auto, None, None).unwrap();

        assert_eq!(target.endpoint_id, "ts");
        assert_eq!(target.destination(), "mac.tailnet.ts.net");
    }

    #[test]
    fn tailscale_first_overrides_online_lan() {
        let endpoints = vec![
            endpoint(
                "lan",
                EndpointKind::LanDns,
                "macbook.local",
                AvailabilityState::Online,
            ),
            endpoint(
                "ts",
                EndpointKind::TailscaleDns,
                "mac.tailnet.ts.net",
                AvailabilityState::Online,
            ),
        ];

        let target = resolve_ssh_target(
            &endpoints,
            EndpointPreference::TailscaleFirst,
            None,
            Some(2222),
        )
        .unwrap();

        assert_eq!(target.endpoint_id, "ts");
        assert_eq!(
            target.command_args(),
            vec!["-p", "2222", "mac.tailnet.ts.net"]
        );
    }

    #[test]
    fn shell_command_quotes_untrusted_arguments() {
        let target = SshTarget {
            endpoint_id: "endpoint-1".into(),
            host: "host name.local".into(),
            port: 2222,
            username: Some("user'; touch /tmp/nope; echo '".into()),
            endpoint_kind: EndpointKind::LanDns,
        };

        assert_eq!(
            target.shell_command(),
            "ssh -p 2222 'user'\"'\"'; touch /tmp/nope; echo '\"'\"'@host name.local'"
        );
    }

    #[test]
    fn proven_endpoint_wins_before_transport_preference() {
        let mut unproven_lan = endpoint(
            "lan",
            EndpointKind::LanIp,
            "192.168.1.25",
            AvailabilityState::Unknown,
        );
        unproven_lan.ssh_capability = AvailabilityState::Unknown;
        let tailscale = endpoint(
            "ts",
            EndpointKind::TailscaleDns,
            "mac.tailnet.ts.net",
            AvailabilityState::Online,
        );

        let target = resolve_ssh_target(
            &[unproven_lan.clone(), tailscale],
            EndpointPreference::LanFirst,
            None,
            None,
        )
        .unwrap();

        assert_eq!(target.endpoint_id, "ts");
        let fallback =
            resolve_ssh_target(&[unproven_lan], EndpointPreference::LanFirst, None, None).unwrap();
        assert_eq!(fallback.endpoint_id, "lan");
    }

    #[test]
    fn availability_state_rejects_obsolete_status_aliases() {
        assert!("stale".parse::<AvailabilityState>().is_err());
        assert!("degraded".parse::<AvailabilityState>().is_err());
    }

    #[test]
    fn endpoint_preference_accepts_legacy_local_first_value() {
        assert_eq!(
            "local_first".parse::<EndpointPreference>().unwrap(),
            EndpointPreference::LanFirst
        );
        assert!("lan_then_tailscale".parse::<EndpointPreference>().is_err());
    }
}
