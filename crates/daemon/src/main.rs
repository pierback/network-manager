use anyhow::{Context, Result};
use clap::Parser;
use network_manager_core::{
    resolve_ssh_target, AvailabilityState, DeviceIdentity as CoreDeviceIdentity,
    EndpointPreference, NetworkEndpoint, TrackedState,
};
use network_manager_db::{
    IdentityLookup, LanDeviceObservation, MdnsServiceObservation, SqliteStore,
    TailscaleNodeObservation,
};
use network_manager_ipc::pb::network_manager_server::{NetworkManager, NetworkManagerServer};
use network_manager_ipc::pb::{
    DaemonStatusResponse, DeviceIdentity, DeviceMutationResponse, DeviceTagRequest,
    DiscoveredDevice, GetDaemonStatusRequest, GetDeviceDetailsRequest, GetDeviceDetailsResponse,
    IdentityCorrectionResponse, ListDeviceIdentitiesRequest, ListDeviceIdentitiesResponse,
    ListDiscoveredDevicesRequest, ListDiscoveredDevicesResponse, MergeIdentitiesRequest,
    NetworkEndpoint as IpcNetworkEndpoint, RefreshRequest, RefreshResponse,
    ResolveSshTargetRequest, ResolveSshTargetResponse, SetDeviceCategoryRequest,
    SetDeviceTextRequest, SetEndpointPreferenceRequest, SetOptionalStringRequest,
    SetSshPortRequest, SetTrackedStateRequest, SplitDiscoveredDeviceRequest,
};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::net::Ipv4Addr;
use std::path::PathBuf;
use std::process::Stdio;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::net::{TcpStream, UnixListener};
use tokio::sync::Mutex as AsyncMutex;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::{Request, Response, Status};

#[derive(Debug, Parser)]
#[command(
    name = "network-manager-daemon",
    about = "Network Manager background daemon"
)]
struct Args {
    /// SQLite database path.
    #[arg(long, env = "NETWORK_MANAGER_DB")]
    db: Option<PathBuf>,

    /// Unix domain socket path for local IPC.
    #[arg(long, env = "NETWORK_MANAGER_SOCKET")]
    socket: Option<PathBuf>,

    /// Seconds between automatic quick refreshes; 0 disables automatic refresh.
    #[arg(
        long,
        env = "NETWORK_MANAGER_REFRESH_INTERVAL_SECONDS",
        default_value_t = 60
    )]
    refresh_interval_seconds: u64,

    /// Disable automatic background quick refreshes.
    #[arg(long, env = "NETWORK_MANAGER_DISABLE_AUTO_REFRESH")]
    disable_auto_refresh: bool,
}

#[derive(Clone)]
struct DaemonService {
    store: Arc<Mutex<SqliteStore>>,
    refresh_lock: Arc<AsyncMutex<()>>,
}

#[tonic::async_trait]
impl NetworkManager for DaemonService {
    async fn get_daemon_status(
        &self,
        _request: Request<GetDaemonStatusRequest>,
    ) -> std::result::Result<Response<DaemonStatusResponse>, Status> {
        let store = self.store.lock().map_err(lock_error)?;
        let status = store.daemon_status("daemon").map_err(internal_error)?;
        Ok(Response::new(DaemonStatusResponse {
            state: status.state,
            source: status.source,
            db_path: status.db_path.unwrap_or_default(),
            started_at: status.started_at.unwrap_or_default(),
            updated_at: status.updated_at.unwrap_or_default(),
            stale: status.stale,
        }))
    }

    async fn refresh(
        &self,
        request: Request<RefreshRequest>,
    ) -> std::result::Result<Response<RefreshResponse>, Status> {
        let _refresh_guard = self.refresh_lock.lock().await;
        let request = request.into_inner();
        let mode = if request.mode.is_empty() {
            "quick".to_string()
        } else {
            request.mode
        };

        let tailscale_result = refresh_tailscale().await;
        let active_probe_result = if mode == "full" {
            Some(active_lan_probe().await)
        } else {
            None
        };
        let mdns_result = refresh_mdns_services().await;
        let lan_result = refresh_lan_arp().await;

        let mut accepted = false;
        let mut targeted_lookup_failed = false;
        let mut messages = Vec::new();
        let mut active_lan_ips = Vec::new();

        let endpoints = {
            let store = self.store.lock().map_err(lock_error)?;
            store.set_daemon_heartbeat().map_err(internal_error)?;

            match tailscale_result {
                Ok(snapshot) => {
                    store
                        .set_metadata("tailscale_service_state", &snapshot.backend_state)
                        .map_err(internal_error)?;
                    let count = store
                        .record_tailscale_nodes(snapshot.tailnet.as_deref(), &snapshot.nodes)
                        .map_err(internal_error)?;
                    accepted = true;
                    messages.push(format!("recorded {count} Tailscale device(s)"));
                }
                Err(error) => {
                    store
                        .set_metadata("tailscale_service_state", "unavailable")
                        .map_err(internal_error)?;
                    messages.push(format!("Tailscale unavailable: {error}"));
                }
            }

            if let Some(active_probe_result) = active_probe_result {
                match active_probe_result {
                    Ok(ips) => {
                        messages.push(format!("probed {} LAN address(es)", ips.len()));
                        active_lan_ips = ips;
                    }
                    Err(error) => messages.push(format!("active LAN probe unavailable: {error}")),
                }
            }

            match mdns_result {
                Ok(observations) => {
                    let count = store
                        .record_mdns_services(&observations)
                        .map_err(internal_error)?;
                    accepted = true;
                    messages.push(format!("recorded {count} mDNS service(s)"));
                }
                Err(error) => messages.push(format!("mDNS unavailable: {error}")),
            }

            match lan_result {
                Ok(observations) => {
                    let count = store
                        .record_lan_devices(&observations)
                        .map_err(internal_error)?;
                    accepted = true;
                    messages.push(format!("recorded {count} LAN ARP device(s)"));
                }
                Err(error) => messages.push(format!("LAN ARP unavailable: {error}")),
            }

            if !active_lan_ips.is_empty() {
                let ip_strings = active_lan_ips
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();
                let changed = store
                    .mark_lan_ips_reachable(&ip_strings)
                    .map_err(internal_error)?;
                messages.push(format!("marked {changed} LAN endpoint(s) reachable"));
            }

            let stale_after_seconds = if mode == "full" { 900 } else { 120 };
            let stale_count = store
                .mark_stale_endpoint_checks_unknown(stale_after_seconds)
                .map_err(internal_error)?;
            if stale_count > 0 {
                messages.push(format!(
                    "marked {stale_count} stale endpoint check(s) unknown"
                ));
            }

            let requested_device = request.device_query.trim();
            if requested_device.is_empty() {
                store
                    .list_endpoints_for_probe(mode != "full")
                    .map_err(internal_error)?
            } else {
                match store
                    .find_identity_id(requested_device)
                    .map_err(internal_error)?
                {
                    IdentityLookup::Found(identity_id) => {
                        let endpoints = store
                            .endpoints_for_identity(&identity_id)
                            .map_err(internal_error)?;
                        messages.push(format!(
                            "selected {} endpoint(s) for device refresh",
                            endpoints.len()
                        ));
                        endpoints
                    }
                    IdentityLookup::NotFound => {
                        targeted_lookup_failed = true;
                        messages.push(format!(
                            "device '{requested_device}' was not found for targeted refresh"
                        ));
                        Vec::new()
                    }
                    IdentityLookup::Ambiguous(ids) => {
                        targeted_lookup_failed = true;
                        messages.push(format!(
                            "device query '{requested_device}' is ambiguous: {}",
                            ids.join(", ")
                        ));
                        Vec::new()
                    }
                }
            }
        };

        let probe_results = probe_ssh_endpoints(endpoints).await;
        if !probe_results.is_empty() {
            let store = self.store.lock().map_err(lock_error)?;
            for result in &probe_results {
                store
                    .set_endpoint_probe_result(
                        &result.endpoint_id,
                        result.reachability,
                        result.ssh_capability,
                    )
                    .map_err(internal_error)?;
            }
            messages.push(format!("probed {} SSH endpoint(s)", probe_results.len()));
        }

        if targeted_lookup_failed {
            accepted = false;
        }

        Ok(Response::new(RefreshResponse {
            accepted,
            message: format!("{mode} refresh: {}", messages.join("; ")),
        }))
    }

    async fn list_device_identities(
        &self,
        request: Request<ListDeviceIdentitiesRequest>,
    ) -> std::result::Result<Response<ListDeviceIdentitiesResponse>, Status> {
        let request = request.into_inner();
        let store = self.store.lock().map_err(lock_error)?;
        let mut identities = store.list_device_identities().map_err(internal_error)?;

        if request.tracked_only {
            identities.retain(|record| record.identity.tracked_state.as_str() == "tracked");
        }
        if request.ignored_only {
            identities.retain(|record| record.identity.tracked_state.as_str() == "ignored");
        }

        Ok(Response::new(ListDeviceIdentitiesResponse {
            identities: identities
                .into_iter()
                .map(|record| DeviceIdentity {
                    id: record.identity.id,
                    stable_key: record.identity.stable_key,
                    label: record.identity.label.unwrap_or_default(),
                    alias: record.identity.alias.unwrap_or_default(),
                    tracked_state: record.identity.tracked_state.as_str().to_string(),
                    category: record.identity.category.unwrap_or_default(),
                    tags: record.identity.tags,
                    ssh_username: record.identity.ssh_username.unwrap_or_default(),
                    ssh_port: record.identity.ssh_port.unwrap_or_default() as u32,
                    endpoint_preference: record.identity.endpoint_preference.as_str().to_string(),
                    last_seen_at: record.identity.last_seen_at.unwrap_or_default(),
                    endpoint_count: record.endpoint_count as u32,
                })
                .collect(),
        }))
    }

    async fn list_discovered_devices(
        &self,
        _request: Request<ListDiscoveredDevicesRequest>,
    ) -> std::result::Result<Response<ListDiscoveredDevicesResponse>, Status> {
        let store = self.store.lock().map_err(lock_error)?;
        let devices = store.list_discovered_devices().map_err(internal_error)?;

        Ok(Response::new(ListDiscoveredDevicesResponse {
            devices: devices
                .into_iter()
                .map(|record| DiscoveredDevice {
                    id: record.device.id,
                    source: record.device.source,
                    source_device_id: record.device.source_device_id,
                    display_name: record.device.display_name.unwrap_or_default(),
                    first_seen_at: record.device.first_seen_at,
                    last_seen_at: record.device.last_seen_at,
                    identity_id: record.identity_id.unwrap_or_default(),
                })
                .collect(),
        }))
    }

    async fn get_device_details(
        &self,
        request: Request<GetDeviceDetailsRequest>,
    ) -> std::result::Result<Response<GetDeviceDetailsResponse>, Status> {
        let query = request.into_inner().device_query;
        let store = self.store.lock().map_err(lock_error)?;
        let identity_id = match lookup_identity_for_rpc(&store, &query)? {
            Ok(identity_id) => identity_id,
            Err(failure) => return Ok(Response::new(details_lookup_failure(failure))),
        };
        let Some(details) = store
            .device_details_by_id(&identity_id)
            .map_err(internal_error)?
        else {
            return Ok(Response::new(GetDeviceDetailsResponse {
                found: false,
                ambiguous: false,
                candidate_identity_ids: vec![identity_id],
                device: None,
                endpoints: Vec::new(),
                message: "device identity disappeared".to_string(),
            }));
        };

        let endpoint_count = details.endpoints.len();
        Ok(Response::new(GetDeviceDetailsResponse {
            found: true,
            ambiguous: false,
            candidate_identity_ids: vec![details.identity.id.clone()],
            device: Some(device_identity_to_ipc(details.identity, endpoint_count)),
            endpoints: details.endpoints.into_iter().map(endpoint_to_ipc).collect(),
            message: "found".to_string(),
        }))
    }

    async fn set_tracked_state(
        &self,
        request: Request<SetTrackedStateRequest>,
    ) -> std::result::Result<Response<DeviceMutationResponse>, Status> {
        let request = request.into_inner();
        let state = TrackedState::from_str(&request.tracked_state)
            .map_err(|error| Status::invalid_argument(format!("invalid tracked_state: {error}")))?;
        self.mutate_device(&request.device_query, |store, identity_id| {
            store.set_tracked_state_by_id(
                identity_id,
                state,
                empty_to_none_str(&request.label),
                empty_to_none_str(&request.alias),
            )
        })
    }

    async fn set_device_label(
        &self,
        request: Request<SetDeviceTextRequest>,
    ) -> std::result::Result<Response<DeviceMutationResponse>, Status> {
        let request = request.into_inner();
        self.mutate_device(&request.device_query, |store, identity_id| {
            store.set_label_by_id(identity_id, &request.value)
        })
    }

    async fn set_device_alias(
        &self,
        request: Request<SetDeviceTextRequest>,
    ) -> std::result::Result<Response<DeviceMutationResponse>, Status> {
        let request = request.into_inner();
        self.mutate_device(&request.device_query, |store, identity_id| {
            store.set_alias_by_id(identity_id, &request.value)
        })
    }

    async fn set_device_category(
        &self,
        request: Request<SetDeviceCategoryRequest>,
    ) -> std::result::Result<Response<DeviceMutationResponse>, Status> {
        let request = request.into_inner();
        let category = if request.clear {
            None
        } else if request.category.trim().is_empty() {
            return Err(Status::invalid_argument(
                "category is required unless clear is true",
            ));
        } else {
            Some(request.category.as_str())
        };
        self.mutate_device(&request.device_query, |store, identity_id| {
            store.set_category_by_id(identity_id, category)
        })
    }

    async fn add_device_tag(
        &self,
        request: Request<DeviceTagRequest>,
    ) -> std::result::Result<Response<DeviceMutationResponse>, Status> {
        let request = request.into_inner();
        self.mutate_device(&request.device_query, |store, identity_id| {
            store.add_tag_by_id(identity_id, &request.tag)
        })
    }

    async fn remove_device_tag(
        &self,
        request: Request<DeviceTagRequest>,
    ) -> std::result::Result<Response<DeviceMutationResponse>, Status> {
        let request = request.into_inner();
        self.mutate_device(&request.device_query, |store, identity_id| {
            store.remove_tag_by_id(identity_id, &request.tag)
        })
    }

    async fn set_ssh_username(
        &self,
        request: Request<SetOptionalStringRequest>,
    ) -> std::result::Result<Response<DeviceMutationResponse>, Status> {
        let request = request.into_inner();
        let username = if request.clear {
            None
        } else {
            Some(request.value.as_str())
        };
        self.mutate_device(&request.device_query, |store, identity_id| {
            store.set_ssh_username_by_id(identity_id, username)
        })
    }

    async fn set_ssh_port(
        &self,
        request: Request<SetSshPortRequest>,
    ) -> std::result::Result<Response<DeviceMutationResponse>, Status> {
        let request = request.into_inner();
        let port = if request.clear {
            None
        } else if request.port == 0 || request.port > u16::MAX as u32 {
            return Err(Status::invalid_argument("invalid SSH port"));
        } else {
            Some(request.port as u16)
        };
        self.mutate_device(&request.device_query, |store, identity_id| {
            store.set_ssh_port_by_id(identity_id, port)
        })
    }

    async fn set_endpoint_preference(
        &self,
        request: Request<SetEndpointPreferenceRequest>,
    ) -> std::result::Result<Response<DeviceMutationResponse>, Status> {
        let request = request.into_inner();
        let preference =
            EndpointPreference::from_str(&request.endpoint_preference).map_err(|error| {
                Status::invalid_argument(format!("invalid endpoint_preference: {error}"))
            })?;
        self.mutate_device(&request.device_query, |store, identity_id| {
            store.set_endpoint_preference_by_id(identity_id, preference)
        })
    }

    async fn merge_identities(
        &self,
        request: Request<MergeIdentitiesRequest>,
    ) -> std::result::Result<Response<IdentityCorrectionResponse>, Status> {
        let request = request.into_inner();
        let store = self.store.lock().map_err(lock_error)?;
        let source_id = match lookup_identity_for_rpc(&store, &request.source_query)? {
            Ok(identity_id) => identity_id,
            Err(failure) => return Ok(Response::new(correction_lookup_failure(failure))),
        };
        let target_id = match lookup_identity_for_rpc(&store, &request.target_query)? {
            Ok(identity_id) => identity_id,
            Err(failure) => return Ok(Response::new(correction_lookup_failure(failure))),
        };
        let result = store
            .merge_identities_by_id(&source_id, &target_id, empty_to_none_str(&request.reason))
            .map_err(internal_error)?;
        Ok(Response::new(correction_success(result)))
    }

    async fn split_discovered_device(
        &self,
        request: Request<SplitDiscoveredDeviceRequest>,
    ) -> std::result::Result<Response<IdentityCorrectionResponse>, Status> {
        let request = request.into_inner();
        let store = self.store.lock().map_err(lock_error)?;
        let result = store
            .split_discovered_device_by_id(
                &request.discovered_device_id,
                empty_to_none_str(&request.reason),
            )
            .map_err(internal_error)?;
        Ok(Response::new(correction_success(result)))
    }

    async fn resolve_ssh_target(
        &self,
        request: Request<ResolveSshTargetRequest>,
    ) -> std::result::Result<Response<ResolveSshTargetResponse>, Status> {
        let request = request.into_inner();
        let preference = if request.endpoint_preference.is_empty() {
            EndpointPreference::Auto
        } else {
            EndpointPreference::from_str(&request.endpoint_preference)
                .unwrap_or(EndpointPreference::Auto)
        };

        let store = self.store.lock().map_err(lock_error)?;
        let identity_id = match store
            .find_identity_id(&request.device_query)
            .map_err(internal_error)?
        {
            IdentityLookup::Found(identity_id) => identity_id,
            IdentityLookup::NotFound => {
                return Ok(Response::new(ResolveSshTargetResponse {
                    found: false,
                    ambiguous: false,
                    message: format!("device '{}' was not found", request.device_query),
                    ..Default::default()
                }))
            }
            IdentityLookup::Ambiguous(ids) => {
                return Ok(Response::new(ResolveSshTargetResponse {
                    found: false,
                    ambiguous: true,
                    candidate_identity_ids: ids,
                    message: format!("device query '{}' is ambiguous", request.device_query),
                    ..Default::default()
                }))
            }
        };

        let identities = store.list_device_identities().map_err(internal_error)?;
        let identity = identities
            .into_iter()
            .find(|record| record.identity.id == identity_id)
            .map(|record| record.identity);
        let endpoints = store
            .endpoints_for_identity(&identity_id)
            .map_err(internal_error)?;
        let username = identity
            .as_ref()
            .and_then(|identity| identity.ssh_username.as_deref());
        let ssh_port = identity.as_ref().and_then(|identity| identity.ssh_port);

        let Some(target) = resolve_ssh_target(&endpoints, preference, username, ssh_port) else {
            return Ok(Response::new(ResolveSshTargetResponse {
                found: false,
                ambiguous: false,
                candidate_identity_ids: vec![identity_id],
                message: "no available SSH target endpoint".to_string(),
                ..Default::default()
            }));
        };

        let ssh_args = target.command_args();
        Ok(Response::new(ResolveSshTargetResponse {
            found: true,
            ambiguous: false,
            candidate_identity_ids: vec![identity_id],
            endpoint_id: target.endpoint_id,
            host: target.host,
            port: target.port as u32,
            username: target.username.unwrap_or_default(),
            endpoint_kind: target.endpoint_kind.as_str().to_string(),
            ssh_args,
            message: "resolved".to_string(),
        }))
    }
}

impl DaemonService {
    #[allow(clippy::result_large_err)]
    fn mutate_device<F>(
        &self,
        query: &str,
        mutate: F,
    ) -> std::result::Result<Response<DeviceMutationResponse>, Status>
    where
        F: FnOnce(&SqliteStore, &str) -> Result<network_manager_db::DeviceMutationResult>,
    {
        let store = self.store.lock().map_err(lock_error)?;
        let identity_id = match lookup_identity_for_rpc(&store, query)? {
            Ok(identity_id) => identity_id,
            Err(failure) => return Ok(Response::new(mutation_lookup_failure(failure))),
        };
        let result = mutate(&store, &identity_id).map_err(internal_error)?;
        Ok(Response::new(mutation_success(result)))
    }
}

#[derive(Debug)]
struct LookupFailure {
    ambiguous: bool,
    candidate_identity_ids: Vec<String>,
    message: String,
}

#[allow(clippy::result_large_err)]
fn lookup_identity_for_rpc(
    store: &SqliteStore,
    query: &str,
) -> std::result::Result<std::result::Result<String, LookupFailure>, Status> {
    Ok(
        match store.find_identity_id(query).map_err(internal_error)? {
            IdentityLookup::Found(identity_id) => Ok(identity_id),
            IdentityLookup::NotFound => Err(LookupFailure {
                ambiguous: false,
                candidate_identity_ids: Vec::new(),
                message: format!("device '{query}' was not found"),
            }),
            IdentityLookup::Ambiguous(ids) => Err(LookupFailure {
                ambiguous: true,
                candidate_identity_ids: ids,
                message: format!("device query '{query}' is ambiguous"),
            }),
        },
    )
}

fn details_lookup_failure(failure: LookupFailure) -> GetDeviceDetailsResponse {
    GetDeviceDetailsResponse {
        found: false,
        ambiguous: failure.ambiguous,
        candidate_identity_ids: failure.candidate_identity_ids,
        device: None,
        endpoints: Vec::new(),
        message: failure.message,
    }
}

fn mutation_lookup_failure(failure: LookupFailure) -> DeviceMutationResponse {
    DeviceMutationResponse {
        found: false,
        ambiguous: failure.ambiguous,
        candidate_identity_ids: failure.candidate_identity_ids,
        device: None,
        message: failure.message,
    }
}

fn correction_lookup_failure(failure: LookupFailure) -> IdentityCorrectionResponse {
    IdentityCorrectionResponse {
        applied: false,
        ambiguous: failure.ambiguous,
        candidate_identity_ids: failure.candidate_identity_ids,
        identity_id: String::new(),
        affected_identity_id: String::new(),
        message: failure.message,
    }
}

fn mutation_success(result: network_manager_db::DeviceMutationResult) -> DeviceMutationResponse {
    DeviceMutationResponse {
        found: true,
        ambiguous: false,
        candidate_identity_ids: vec![result.identity.id.clone()],
        device: Some(device_identity_to_ipc(
            result.identity,
            result.endpoint_count,
        )),
        message: result.message,
    }
}

fn correction_success(
    result: network_manager_db::IdentityCorrectionResult,
) -> IdentityCorrectionResponse {
    IdentityCorrectionResponse {
        applied: true,
        ambiguous: false,
        candidate_identity_ids: Vec::new(),
        identity_id: result.identity_id,
        affected_identity_id: result.affected_identity_id,
        message: result.message,
    }
}

fn device_identity_to_ipc(identity: CoreDeviceIdentity, endpoint_count: usize) -> DeviceIdentity {
    DeviceIdentity {
        id: identity.id,
        stable_key: identity.stable_key,
        label: identity.label.unwrap_or_default(),
        alias: identity.alias.unwrap_or_default(),
        tracked_state: identity.tracked_state.as_str().to_string(),
        category: identity.category.unwrap_or_default(),
        tags: identity.tags,
        ssh_username: identity.ssh_username.unwrap_or_default(),
        ssh_port: identity.ssh_port.unwrap_or_default() as u32,
        endpoint_preference: identity.endpoint_preference.as_str().to_string(),
        last_seen_at: identity.last_seen_at.unwrap_or_default(),
        endpoint_count: endpoint_count as u32,
    }
}

fn endpoint_to_ipc(endpoint: NetworkEndpoint) -> IpcNetworkEndpoint {
    IpcNetworkEndpoint {
        id: endpoint.id,
        identity_id: endpoint.identity_id,
        kind: endpoint.kind.as_str().to_string(),
        address: endpoint.address,
        port: endpoint.port.unwrap_or_default() as u32,
        hostname: endpoint.hostname.unwrap_or_default(),
        reachability: endpoint.reachability.as_str().to_string(),
        ssh_capability: endpoint.ssh_capability.as_str().to_string(),
        last_seen_at: endpoint.last_seen_at.unwrap_or_default(),
        last_checked_at: endpoint.last_checked_at.unwrap_or_default(),
    }
}

fn empty_to_none_str(value: &str) -> Option<&str> {
    (!value.is_empty()).then_some(value)
}

#[derive(Debug)]
struct TailscaleSnapshot {
    backend_state: String,
    tailnet: Option<String>,
    nodes: Vec<TailscaleNodeObservation>,
}

#[derive(Debug, Deserialize)]
struct TailscaleStatusJson {
    #[serde(rename = "BackendState")]
    backend_state: Option<String>,
    #[serde(rename = "CurrentTailnet")]
    current_tailnet: Option<serde_json::Value>,
    #[serde(rename = "MagicDNSSuffix")]
    magic_dns_suffix: Option<String>,
    #[serde(rename = "Self")]
    self_peer: Option<TailscalePeerJson>,
    #[serde(rename = "Peer", default)]
    peers: HashMap<String, TailscalePeerJson>,
}

#[derive(Debug, Deserialize, serde::Serialize)]
struct TailscalePeerJson {
    #[serde(rename = "ID")]
    id: Option<serde_json::Value>,
    #[serde(rename = "PublicKey")]
    public_key: Option<String>,
    #[serde(rename = "HostName")]
    host_name: Option<String>,
    #[serde(rename = "DNSName")]
    dns_name: Option<String>,
    #[serde(rename = "TailscaleIPs", default)]
    tailscale_ips: Vec<String>,
    #[serde(rename = "Online")]
    online: Option<bool>,
    #[serde(rename = "OS")]
    os: Option<String>,
}

async fn refresh_tailscale() -> Result<TailscaleSnapshot> {
    let output = tokio::time::timeout(
        Duration::from_secs(5),
        tokio::process::Command::new("tailscale")
            .args(["status", "--json"])
            .output(),
    )
    .await
    .context("tailscale status timed out")?
    .context("running tailscale status --json")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        anyhow::bail!(
            "tailscale status failed{}",
            if stderr.is_empty() {
                String::new()
            } else {
                format!(": {stderr}")
            }
        );
    }

    let status: TailscaleStatusJson =
        serde_json::from_slice(&output.stdout).context("parsing tailscale status --json output")?;
    let tailnet = status
        .magic_dns_suffix
        .clone()
        .or_else(|| tailnet_name(status.current_tailnet.as_ref()));
    let mut nodes = Vec::new();

    if let Some(self_peer) = status.self_peer {
        nodes.push(peer_to_observation("self", self_peer)?);
    }

    for (key, peer) in status.peers {
        nodes.push(peer_to_observation(&key, peer)?);
    }

    Ok(TailscaleSnapshot {
        backend_state: status
            .backend_state
            .unwrap_or_else(|| "unknown".to_string()),
        tailnet,
        nodes,
    })
}

fn peer_to_observation(
    fallback_key: &str,
    peer: TailscalePeerJson,
) -> Result<TailscaleNodeObservation> {
    let source_device_id = peer
        .id
        .as_ref()
        .and_then(value_to_string)
        .or_else(|| peer.public_key.clone())
        .unwrap_or_else(|| fallback_key.to_string());
    let raw_json = serde_json::to_string(&peer).context("serializing tailscale peer evidence")?;
    Ok(TailscaleNodeObservation {
        source_device_id,
        display_name: peer.host_name,
        dns_name: peer
            .dns_name
            .map(|name| name.trim_end_matches('.').to_string()),
        tailscale_ips: peer.tailscale_ips,
        online: peer.online,
        os: peer.os,
        raw_json,
    })
}

fn value_to_string(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(value) => Some(value.clone()),
        serde_json::Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

fn tailnet_name(value: Option<&serde_json::Value>) -> Option<String> {
    match value? {
        serde_json::Value::String(value) => Some(value.clone()),
        serde_json::Value::Object(object) => object
            .get("Name")
            .and_then(value_to_string)
            .or_else(|| object.get("MagicDNSSuffix").and_then(value_to_string)),
        _ => None,
    }
}

async fn active_lan_probe() -> Result<Vec<Ipv4Addr>> {
    let subnets = local_ipv4_subnets().await?;
    let mut targets = Vec::new();
    for (address, netmask) in subnets {
        targets.extend(expand_probe_targets(address, netmask));
    }
    targets.sort_unstable();
    targets.dedup();

    let mut successful = Vec::new();
    for chunk in targets.chunks(32) {
        let handles = chunk
            .iter()
            .map(|ip| {
                let ip = *ip;
                tokio::spawn(async move { ping_once(ip).await })
            })
            .collect::<Vec<_>>();
        for handle in handles {
            if let Ok(Some(ip)) = handle.await {
                successful.push(ip);
            }
        }
    }

    Ok(successful)
}

async fn local_ipv4_subnets() -> Result<Vec<(Ipv4Addr, Ipv4Addr)>> {
    let output = tokio::time::timeout(
        Duration::from_secs(3),
        tokio::process::Command::new("ifconfig").output(),
    )
    .await
    .context("ifconfig timed out")?
    .context("running ifconfig")?;

    if !output.status.success() {
        anyhow::bail!("ifconfig failed");
    }

    let text = String::from_utf8_lossy(&output.stdout);
    Ok(text.lines().filter_map(parse_ifconfig_inet_line).collect())
}

fn parse_ifconfig_inet_line(line: &str) -> Option<(Ipv4Addr, Ipv4Addr)> {
    let line = line.trim();
    if !line.starts_with("inet ") {
        return None;
    }
    let parts = line.split_whitespace().collect::<Vec<_>>();
    let address = parts.get(1)?.parse::<Ipv4Addr>().ok()?;
    if address.is_loopback()
        || address.is_link_local()
        || address.is_multicast()
        || is_tailscale_cgnat(address)
    {
        return None;
    }
    let netmask_index = parts.iter().position(|part| *part == "netmask")?;
    let netmask_text = *parts.get(netmask_index + 1)?;
    let netmask = parse_ifconfig_netmask(netmask_text)?;
    Some((address, netmask))
}

fn parse_ifconfig_netmask(value: &str) -> Option<Ipv4Addr> {
    if let Some(hex) = value.strip_prefix("0x") {
        let raw = u32::from_str_radix(hex, 16).ok()?;
        Some(Ipv4Addr::from(raw))
    } else {
        value.parse().ok()
    }
}

fn expand_probe_targets(address: Ipv4Addr, netmask: Ipv4Addr) -> Vec<Ipv4Addr> {
    let address = u32::from(address);
    let mut mask = u32::from(netmask);
    let prefix = mask.count_ones();
    if prefix < 24 {
        mask = u32::from(Ipv4Addr::new(255, 255, 255, 0));
    }

    let network = address & mask;
    let broadcast = network | !mask;
    let host_count = broadcast.saturating_sub(network).saturating_sub(1);
    if host_count == 0 || host_count > 254 {
        return Vec::new();
    }

    ((network + 1)..broadcast)
        .map(Ipv4Addr::from)
        .filter(|ip| u32::from(*ip) != address)
        .collect()
}

async fn ping_once(ip: Ipv4Addr) -> Option<Ipv4Addr> {
    let status = tokio::time::timeout(
        Duration::from_millis(700),
        tokio::process::Command::new("ping")
            .args(["-c", "1", "-W", "200", &ip.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status(),
    )
    .await;

    matches!(status, Ok(Ok(status)) if status.success()).then_some(ip)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MdnsBrowseService {
    service_name: String,
    service_type: String,
    domain: String,
    raw_line: String,
}

async fn refresh_mdns_services() -> Result<Vec<MdnsServiceObservation>> {
    let service_types = [
        "_ssh._tcp",
        "_device-info._tcp",
        "_workstation._tcp",
        "_smb._tcp",
        "_ipp._tcp",
        "_ipps._tcp",
        "_printer._tcp",
        "_http._tcp",
    ];

    let browse_handles = service_types
        .iter()
        .map(|service_type| {
            let args = vec![
                "-B".to_string(),
                (*service_type).to_string(),
                "local.".to_string(),
            ];
            tokio::spawn(async move { capture_dns_sd(&args, Duration::from_millis(900)).await })
        })
        .collect::<Vec<_>>();

    let mut services = Vec::new();
    for handle in browse_handles {
        if let Ok(Ok(output)) = handle.await {
            services.extend(parse_dns_sd_browse(&output));
        }
    }

    let mut seen = HashSet::new();
    services.retain(|service| {
        seen.insert(format!(
            "{}:{}:{}",
            service.domain, service.service_type, service.service_name
        ))
    });

    let mut observations = Vec::new();
    for chunk in services.chunks(12).take(3) {
        let resolve_handles = chunk
            .iter()
            .map(|service| {
                let service = service.clone();
                tokio::spawn(async move {
                    let resolve_output = capture_dns_sd(
                        &[
                            "-L".to_string(),
                            service.service_name.clone(),
                            service.service_type.clone(),
                            format!("{}.", service.domain),
                        ],
                        Duration::from_millis(900),
                    )
                    .await
                    .unwrap_or_default();
                    (service, resolve_output)
                })
            })
            .collect::<Vec<_>>();

        for handle in resolve_handles {
            let Ok((service, resolve_output)) = handle.await else {
                continue;
            };
            let (hostname, port) = parse_dns_sd_resolve(&resolve_output)
                .map(|(hostname, port)| (Some(hostname), Some(port)))
                .unwrap_or((None, None));
            let raw_text = if resolve_output.trim().is_empty() {
                service.raw_line.clone()
            } else {
                format!("{}\n{}", service.raw_line, resolve_output.trim())
            };

            observations.push(MdnsServiceObservation {
                source_device_id: format!(
                    "{}:{}:{}",
                    service.domain, service.service_type, service.service_name
                ),
                service_name: service.service_name,
                service_type: service.service_type,
                domain: service.domain,
                hostname,
                port,
                raw_text,
            });
        }
    }

    Ok(observations)
}

async fn capture_dns_sd(args: &[String], duration: Duration) -> Result<String> {
    let mut command = tokio::process::Command::new("dns-sd");
    command
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = command
        .spawn()
        .with_context(|| format!("running dns-sd {}", args.join(" ")))?;

    tokio::time::sleep(duration).await;
    let _ = child.start_kill();
    let output = tokio::time::timeout(Duration::from_secs(1), child.wait_with_output())
        .await
        .context("waiting for dns-sd to exit")?
        .context("reading dns-sd output")?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stdout.trim().is_empty() && !stderr.is_empty() {
        anyhow::bail!("dns-sd {} failed: {stderr}", args.join(" "));
    }
    Ok(stdout)
}

fn parse_dns_sd_browse(output: &str) -> Vec<MdnsBrowseService> {
    let mut seen = HashSet::new();
    let mut services = Vec::new();

    for line in output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let parts = line.split_whitespace().collect::<Vec<_>>();
        let Some(action_index) = parts.iter().position(|part| *part == "Add") else {
            continue;
        };
        if parts.len() <= action_index + 5 {
            continue;
        }
        let domain = trim_dns_sd_field(parts[action_index + 3]);
        let service_type = trim_dns_sd_field(parts[action_index + 4]);
        let service_name = parts[action_index + 5..].join(" ");
        if service_name.is_empty() {
            continue;
        }
        let key = format!("{domain}:{service_type}:{service_name}");
        if seen.insert(key) {
            services.push(MdnsBrowseService {
                service_name,
                service_type,
                domain,
                raw_line: line.to_string(),
            });
        }
    }

    services
}

fn parse_dns_sd_resolve(output: &str) -> Option<(String, u16)> {
    for line in output.lines() {
        let Some((_, rest)) = line.split_once(" can be reached at ") else {
            continue;
        };
        let host_port = rest.split(" (").next().unwrap_or(rest).trim();
        let Some((hostname, port)) = host_port.rsplit_once(':') else {
            continue;
        };
        let hostname = hostname.trim().trim_end_matches('.').to_string();
        let Ok(port) = port.trim().parse::<u16>() else {
            continue;
        };
        if !hostname.is_empty() {
            return Some((hostname, port));
        }
    }
    None
}

fn trim_dns_sd_field(value: &str) -> String {
    value.trim().trim_end_matches('.').to_string()
}

#[derive(Debug)]
struct EndpointProbeResult {
    endpoint_id: String,
    reachability: Option<AvailabilityState>,
    ssh_capability: AvailabilityState,
}

async fn probe_ssh_endpoints(endpoints: Vec<NetworkEndpoint>) -> Vec<EndpointProbeResult> {
    let mut results = Vec::new();
    for chunk in endpoints.chunks(32) {
        let handles = chunk
            .iter()
            .map(|endpoint| {
                let endpoint = endpoint.clone();
                tokio::spawn(async move { probe_ssh_endpoint(endpoint).await })
            })
            .collect::<Vec<_>>();
        for handle in handles {
            if let Ok(result) = handle.await {
                results.push(result);
            }
        }
    }
    results
}

async fn probe_ssh_endpoint(endpoint: NetworkEndpoint) -> EndpointProbeResult {
    let host = endpoint.host_for_connection().to_string();
    let port = endpoint.port.unwrap_or(22);
    let target = format!("{host}:{port}");
    let result =
        tokio::time::timeout(Duration::from_millis(900), TcpStream::connect(&target)).await;

    match result {
        Ok(Ok(_stream)) => EndpointProbeResult {
            endpoint_id: endpoint.id,
            reachability: Some(AvailabilityState::Online),
            ssh_capability: AvailabilityState::Online,
        },
        Ok(Err(error)) if error.kind() == std::io::ErrorKind::ConnectionRefused => {
            EndpointProbeResult {
                endpoint_id: endpoint.id,
                reachability: Some(AvailabilityState::Online),
                ssh_capability: AvailabilityState::Offline,
            }
        }
        _ => EndpointProbeResult {
            endpoint_id: endpoint.id,
            reachability: None,
            ssh_capability: AvailabilityState::Offline,
        },
    }
}

async fn refresh_lan_arp() -> Result<Vec<LanDeviceObservation>> {
    let output = tokio::time::timeout(
        Duration::from_secs(3),
        tokio::process::Command::new("arp").arg("-an").output(),
    )
    .await
    .context("arp scan timed out")?
    .context("running arp -an")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        anyhow::bail!(
            "arp -an failed{}",
            if stderr.is_empty() {
                String::new()
            } else {
                format!(": {stderr}")
            }
        );
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let mut observations = text.lines().filter_map(parse_arp_line).collect::<Vec<_>>();
    resolve_lan_hostnames(&mut observations).await;
    Ok(observations)
}

async fn resolve_lan_hostnames(observations: &mut [LanDeviceObservation]) {
    let handles = observations
        .iter()
        .enumerate()
        .filter(|(_, observation)| observation.hostname.is_none())
        .map(|(index, observation)| {
            let ip_address = observation.ip_address.clone();
            tokio::spawn(async move { (index, reverse_dns_lookup(&ip_address).await) })
        })
        .collect::<Vec<_>>();

    for handle in handles {
        let Ok((index, Some(hostname))) = handle.await else {
            continue;
        };
        if let Some(observation) = observations.get_mut(index) {
            observation.hostname = Some(hostname);
        }
    }
}

async fn reverse_dns_lookup(ip_address: &str) -> Option<String> {
    let output = tokio::time::timeout(
        Duration::from_millis(700),
        tokio::process::Command::new("dscacheutil")
            .args(["-q", "host", "-a", "ip_address", ip_address])
            .output(),
    )
    .await
    .ok()?
    .ok()?;
    if !output.status.success() {
        return None;
    }
    parse_dscacheutil_hostname(&String::from_utf8_lossy(&output.stdout))
}

fn parse_dscacheutil_hostname(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        (key.trim() == "name")
            .then(|| value.trim().trim_end_matches('.').to_string())
            .filter(|value| !value.is_empty())
    })
}

fn parse_arp_line(line: &str) -> Option<LanDeviceObservation> {
    let open = line.find('(')?;
    let close = line[open + 1..].find(')')? + open + 1;
    let ip_address = line[open + 1..close].to_string();
    if is_multicast_or_broadcast(&ip_address) {
        return None;
    }

    let hostname = line[..open].trim();
    let hostname = if hostname.is_empty() || hostname == "?" {
        None
    } else {
        Some(hostname.to_string())
    };

    let after = &line[close + 1..];
    let at_marker = " at ";
    let at_index = after.find(at_marker)? + at_marker.len();
    let after_at = &after[at_index..];
    let mac = after_at.split_whitespace().next()?.trim();
    let mac_address = if mac == "(incomplete)" || is_multicast_mac(mac) {
        None
    } else {
        Some(mac.to_ascii_lowercase().replace('-', ":"))
    };

    let interface_name = after_at
        .split_whitespace()
        .collect::<Vec<_>>()
        .windows(2)
        .find_map(|window| (window[0] == "on").then(|| window[1].to_string()));

    if mac_address.is_none() && hostname.is_none() {
        return None;
    }

    Some(LanDeviceObservation {
        ip_address,
        hostname,
        mac_address,
        interface_name,
        raw_text: line.to_string(),
    })
}

fn is_tailscale_cgnat(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();
    octets[0] == 100 && (64..=127).contains(&octets[1])
}

fn is_multicast_or_broadcast(ip: &str) -> bool {
    ip == "255.255.255.255"
        || ip.starts_with("224.")
        || ip.starts_with("225.")
        || ip.starts_with("226.")
        || ip.starts_with("227.")
        || ip.starts_with("228.")
        || ip.starts_with("229.")
        || ip.starts_with("230.")
        || ip.starts_with("231.")
        || ip.starts_with("232.")
        || ip.starts_with("233.")
        || ip.starts_with("234.")
        || ip.starts_with("235.")
        || ip.starts_with("236.")
        || ip.starts_with("237.")
        || ip.starts_with("238.")
        || ip.starts_with("239.")
}

fn is_multicast_mac(mac: &str) -> bool {
    let normalized = mac.to_ascii_lowercase().replace('-', ":");
    normalized == "ff:ff:ff:ff:ff:ff"
        || normalized.starts_with("01:00:5e")
        || normalized.starts_with("33:33")
}

fn spawn_auto_refresh(socket_path: PathBuf, interval: Duration) {
    tokio::spawn(async move {
        let initial_delay = interval.min(Duration::from_secs(5));
        tokio::time::sleep(initial_delay).await;

        loop {
            match network_manager_ipc::connect_uds(&socket_path).await {
                Ok(mut client) => match client
                    .refresh(RefreshRequest {
                        mode: "quick".to_string(),
                        device_query: String::new(),
                    })
                    .await
                {
                    Ok(response) => {
                        println!("auto refresh: {}", response.into_inner().message);
                    }
                    Err(error) => eprintln!("auto refresh failed: {error:#}"),
                },
                Err(error) => eprintln!("auto refresh could not connect to daemon: {error:#}"),
            }
            tokio::time::sleep(interval).await;
        }
    });
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let refresh_interval_seconds = args.refresh_interval_seconds;
    let disable_auto_refresh = args.disable_auto_refresh;
    let db_path = args.db.unwrap_or_else(network_manager_db::default_db_path);
    let socket_path = args
        .socket
        .unwrap_or_else(network_manager_ipc::default_socket_path);

    let store = SqliteStore::open(&db_path)?;
    store.migrate()?;
    store.set_daemon_started()?;
    store.set_metadata(
        "auto_refresh_enabled",
        if disable_auto_refresh || refresh_interval_seconds == 0 {
            "false"
        } else {
            "true"
        },
    )?;
    store.set_metadata(
        "auto_refresh_interval_seconds",
        &refresh_interval_seconds.to_string(),
    )?;

    if let Some(parent) = network_manager_ipc::socket_parent(&socket_path) {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating socket directory {}", parent.display()))?;
    }
    if socket_path.exists() {
        std::fs::remove_file(&socket_path)
            .with_context(|| format!("removing stale socket {}", socket_path.display()))?;
    }

    let listener = UnixListener::bind(&socket_path)
        .with_context(|| format!("binding socket {}", socket_path.display()))?;
    secure_socket_permissions(&socket_path)?;

    let service = DaemonService {
        store: Arc::new(Mutex::new(store)),
        refresh_lock: Arc::new(AsyncMutex::new(())),
    };

    if !disable_auto_refresh && refresh_interval_seconds > 0 {
        spawn_auto_refresh(
            socket_path.clone(),
            Duration::from_secs(refresh_interval_seconds),
        );
    }

    println!(
        "network-manager-daemon listening on {}",
        socket_path.display()
    );
    tonic::transport::Server::builder()
        .add_service(NetworkManagerServer::new(service))
        .serve_with_incoming(UnixListenerStream::new(listener))
        .await
        .context("serving network-manager daemon")?;

    Ok(())
}

fn internal_error(error: anyhow::Error) -> Status {
    Status::internal(error.to_string())
}

fn lock_error<T>(error: std::sync::PoisonError<T>) -> Status {
    Status::internal(format!("daemon store lock poisoned: {error}"))
}

#[cfg(unix)]
fn secure_socket_permissions(path: &std::path::Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = std::fs::metadata(path)?.permissions();
    permissions.set_mode(0o600);
    std::fs::set_permissions(path, permissions)?;
    Ok(())
}

#[cfg(not(unix))]
fn secure_socket_permissions(_path: &std::path::Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dns_sd_browse_services_with_spaces() {
        let output = "Browsing for _ssh._tcp.local.\nTimestamp     A/R    Flags  if Domain               Service Type         Instance Name\n13:05:01.000  Add        3  14 local.               _ssh._tcp.          Office MacBook Pro\n13:05:02.000  Rmv        0  14 local.               _ssh._tcp.          Old Host\n";

        let services = parse_dns_sd_browse(output);

        assert_eq!(services.len(), 1);
        assert_eq!(services[0].domain, "local");
        assert_eq!(services[0].service_type, "_ssh._tcp");
        assert_eq!(services[0].service_name, "Office MacBook Pro");
    }

    #[test]
    fn parses_dscacheutil_reverse_hostname() {
        assert_eq!(
            parse_dscacheutil_hostname("name: office-macbook.local\nip_address: 192.168.1.20"),
            Some("office-macbook.local".to_string())
        );
    }

    #[test]
    fn parses_dns_sd_resolve_target() {
        let output = "Lookup Office MacBook Pro._ssh._tcp.local.\nOffice MacBook Pro._ssh._tcp.local. can be reached at office-macbook.local.:2222 (interface 14)\n";

        assert_eq!(
            parse_dns_sd_resolve(output),
            Some(("office-macbook.local".to_string(), 2222))
        );
    }
}
