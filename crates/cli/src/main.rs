use anyhow::{anyhow, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use network_manager_core::{resolve_ssh_target, EndpointPreference, SshTarget, TrackedState};
use network_manager_db::{DeviceMutationResult, IdentityLookup, SqliteStore};
use network_manager_ipc::pb::{
    GetDaemonStatusRequest, ListDeviceIdentitiesRequest, ListDiscoveredDevicesRequest,
    RefreshRequest, ResolveSshTargetRequest,
};
use serde::Serialize;
use std::path::PathBuf;

const EXIT_NOT_FOUND: i32 = 20;
const EXIT_AMBIGUOUS: i32 = 21;
const EXIT_UNAVAILABLE: i32 = 22;
const EXIT_DAEMON_DOWN: i32 = 23;

#[derive(Debug, Parser)]
#[command(name = "network-manager", about = "Agent-friendly Network Manager CLI")]
struct Cli {
    /// SQLite database path.
    #[arg(long, env = "NETWORK_MANAGER_DB", global = true)]
    db: Option<PathBuf>,

    /// Unix domain socket path for daemon IPC.
    #[arg(long, env = "NETWORK_MANAGER_SOCKET", global = true)]
    socket: Option<PathBuf>,

    /// Emit JSON output.
    #[arg(long, global = true)]
    json: bool,

    /// Do not contact the daemon; read SQLite only where possible.
    #[arg(long, global = true)]
    offline: bool,

    /// Fail instead of falling back to SQLite if the daemon is unavailable.
    #[arg(long, global = true)]
    require_daemon: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Print configured local paths.
    Paths(PathsArgs),
    /// Daemon-related commands.
    Daemon {
        #[command(subcommand)]
        command: DaemonCommand,
    },
    /// Device identity commands.
    Devices {
        #[command(subcommand)]
        command: DevicesCommand,
    },
    /// List known device identities.
    List(DeviceListArgs),
    /// Show one device identity with endpoints.
    Show(DeviceQueryArgs),
    /// Mark a discovered identity as tracked/favorite.
    Track(TrackArgs),
    /// Mark a device identity as untracked.
    Untrack(DeviceQueryArgs),
    /// Ignore a noisy device identity.
    Ignore(DeviceQueryArgs),
    /// Set the human-facing label.
    Label(LabelArgs),
    /// Set the CLI-friendly unique alias.
    Alias(AliasArgs),
    /// Discovery commands.
    Discover {
        #[command(subcommand)]
        command: DiscoverCommand,
    },
    /// Resolve a device to an SSH target.
    Resolve(ResolveArgs),
    /// Request a bounded daemon refresh.
    Refresh(RefreshArgs),
    /// Exec into ssh for the resolved device.
    Ssh(SshArgs),
}

#[derive(Debug, Args)]
struct PathsArgs {
    #[arg(value_enum)]
    kind: PathKind,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum PathKind {
    Db,
    Socket,
}

#[derive(Debug, Subcommand)]
enum DaemonCommand {
    /// Show daemon status.
    Status,
}

#[derive(Debug, Subcommand)]
enum DevicesCommand {
    /// List known device identities.
    List(DeviceListArgs),
    /// Show one device identity with endpoints.
    Show(DeviceQueryArgs),
    /// Mark a discovered identity as tracked/favorite.
    Track(TrackArgs),
    /// Mark a device identity as untracked.
    Untrack(DeviceQueryArgs),
    /// Ignore a noisy device identity.
    Ignore(DeviceQueryArgs),
    /// Set the human-facing label.
    Label(LabelArgs),
    /// Set the CLI-friendly unique alias.
    Alias(AliasArgs),
    /// Set or clear the device SSH username.
    SshUser(SshUserArgs),
    /// Set or clear the device SSH port.
    SshPort(SshPortArgs),
    /// Set endpoint preference for SSH resolution.
    Preference(PreferenceArgs),
}

#[derive(Debug, Args)]
struct DeviceListArgs {
    #[arg(long)]
    tracked: bool,
    #[arg(long)]
    ignored: bool,
}

#[derive(Debug, Args)]
struct DeviceQueryArgs {
    device: String,
}

#[derive(Debug, Args)]
struct TrackArgs {
    device: String,
    #[arg(long)]
    label: Option<String>,
    #[arg(long)]
    alias: Option<String>,
}

#[derive(Debug, Args)]
struct LabelArgs {
    device: String,
    label: String,
}

#[derive(Debug, Args)]
struct AliasArgs {
    device: String,
    alias: String,
}

#[derive(Debug, Args)]
struct SshUserArgs {
    device: String,
    /// Username to store; omit with --clear to remove.
    username: Option<String>,
    #[arg(long)]
    clear: bool,
}

#[derive(Debug, Args)]
struct SshPortArgs {
    device: String,
    /// SSH port to store; omit with --clear to remove.
    port: Option<u16>,
    #[arg(long)]
    clear: bool,
}

#[derive(Debug, Args)]
struct PreferenceArgs {
    device: String,
    #[arg(value_enum)]
    preference: CliPreference,
}

#[derive(Debug, Subcommand)]
enum DiscoverCommand {
    /// List discovered devices.
    List,
}

#[derive(Debug, Args)]
struct ResolveArgs {
    device: String,
    #[arg(long, value_enum, default_value_t = CliPreference::Auto)]
    preference: CliPreference,
    /// Print a full ssh command instead of just the destination.
    #[arg(long)]
    ssh_command: bool,
}

#[derive(Debug, Args)]
struct RefreshArgs {
    #[arg(long, conflicts_with = "full")]
    quick: bool,
    #[arg(long)]
    full: bool,
    #[arg(long)]
    device: Option<String>,
}

#[derive(Debug, Args)]
struct SshArgs {
    device: String,
    #[arg(long, value_enum, default_value_t = CliPreference::Auto)]
    preference: CliPreference,
    /// Extra arguments/remote command passed to ssh after the destination.
    #[arg(last = true)]
    ssh_args: Vec<String>,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliPreference {
    Auto,
    LocalFirst,
    TailscaleFirst,
    LanFirst,
}

impl CliPreference {
    fn as_core(self) -> EndpointPreference {
        match self {
            Self::Auto => EndpointPreference::Auto,
            Self::LocalFirst => EndpointPreference::LocalFirst,
            Self::TailscaleFirst => EndpointPreference::TailscaleFirst,
            Self::LanFirst => EndpointPreference::LanFirst,
        }
    }
}

#[derive(Debug, Serialize)]
struct StatusOutput {
    state: String,
    source: String,
    db_path: Option<String>,
    started_at: Option<String>,
    updated_at: Option<String>,
    stale: bool,
    daemon_available: bool,
}

#[derive(Debug, Serialize)]
struct DeviceOutput {
    id: String,
    stable_key: String,
    label: Option<String>,
    alias: Option<String>,
    tracked_state: String,
    category: Option<String>,
    tags: Vec<String>,
    ssh_username: Option<String>,
    ssh_port: Option<u16>,
    endpoint_preference: String,
    last_seen_at: Option<String>,
    endpoint_count: usize,
}

#[derive(Debug, Serialize)]
struct EndpointOutput {
    id: String,
    kind: String,
    address: String,
    port: Option<u16>,
    hostname: Option<String>,
    reachability: String,
    ssh_capability: String,
    last_seen_at: Option<String>,
    last_checked_at: Option<String>,
}

#[derive(Debug, Serialize)]
struct DeviceDetailsOutput {
    device: DeviceOutput,
    endpoints: Vec<EndpointOutput>,
}

#[derive(Debug, Serialize)]
struct MutationOutput {
    message: String,
    device: DeviceOutput,
}

#[derive(Debug, Serialize)]
struct DiscoveredOutput {
    id: String,
    source: String,
    source_device_id: String,
    display_name: Option<String>,
    first_seen_at: String,
    last_seen_at: String,
    identity_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct ResolveOutput {
    found: bool,
    ambiguous: bool,
    source: String,
    candidate_identity_ids: Vec<String>,
    endpoint_id: Option<String>,
    host: Option<String>,
    port: Option<u16>,
    username: Option<String>,
    endpoint_kind: Option<String>,
    ssh_args: Vec<String>,
    message: String,
}

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();
    let paths = Paths {
        db: cli
            .db
            .clone()
            .unwrap_or_else(network_manager_db::default_db_path),
        socket: cli
            .socket
            .clone()
            .unwrap_or_else(network_manager_ipc::default_socket_path),
    };

    match &cli.command {
        Command::Paths(args) => {
            match args.kind {
                PathKind::Db => print_value(&paths.db.display().to_string(), cli.json),
                PathKind::Socket => print_value(&paths.socket.display().to_string(), cli.json),
            }?;
        }
        Command::Daemon {
            command: DaemonCommand::Status,
        } => {
            let status = daemon_status(&cli, &paths).await?;
            print_struct(&status, cli.json)?;
        }
        Command::Devices { command } => match command {
            DevicesCommand::List(args) => {
                let devices = list_devices(&cli, &paths, args).await?;
                print_struct(&devices, cli.json)?;
            }
            DevicesCommand::Show(args) => {
                let details = show_device(&paths, &args.device)?;
                print_struct(&details, cli.json)?;
            }
            DevicesCommand::Track(args) => {
                let result = mutate_tracked_state(
                    &paths,
                    &args.device,
                    TrackedState::Tracked,
                    args.label.as_deref(),
                    args.alias.as_deref(),
                )?;
                print_mutation(result, cli.json)?;
            }
            DevicesCommand::Untrack(args) => {
                let result = mutate_tracked_state(
                    &paths,
                    &args.device,
                    TrackedState::Untracked,
                    None,
                    None,
                )?;
                print_mutation(result, cli.json)?;
            }
            DevicesCommand::Ignore(args) => {
                let result =
                    mutate_tracked_state(&paths, &args.device, TrackedState::Ignored, None, None)?;
                print_mutation(result, cli.json)?;
            }
            DevicesCommand::Label(args) => {
                let store = open_store(&paths.db)?;
                let identity_id = lookup_or_exit(&store, &args.device)?;
                let result = store.set_label_by_id(&identity_id, &args.label)?;
                print_mutation(result, cli.json)?;
            }
            DevicesCommand::Alias(args) => {
                let store = open_store(&paths.db)?;
                let identity_id = lookup_or_exit(&store, &args.device)?;
                let result = store.set_alias_by_id(&identity_id, &args.alias)?;
                print_mutation(result, cli.json)?;
            }
            DevicesCommand::SshUser(args) => {
                let store = open_store(&paths.db)?;
                let identity_id = lookup_or_exit(&store, &args.device)?;
                let username = if args.clear {
                    None
                } else {
                    args.username.as_deref()
                };
                let result = store.set_ssh_username_by_id(&identity_id, username)?;
                print_mutation(result, cli.json)?;
            }
            DevicesCommand::SshPort(args) => {
                let store = open_store(&paths.db)?;
                let identity_id = lookup_or_exit(&store, &args.device)?;
                let port = if args.clear { None } else { args.port };
                let result = store.set_ssh_port_by_id(&identity_id, port)?;
                print_mutation(result, cli.json)?;
            }
            DevicesCommand::Preference(args) => {
                let store = open_store(&paths.db)?;
                let identity_id = lookup_or_exit(&store, &args.device)?;
                let result =
                    store.set_endpoint_preference_by_id(&identity_id, args.preference.as_core())?;
                print_mutation(result, cli.json)?;
            }
        },
        Command::List(args) => {
            let devices = list_devices(&cli, &paths, args).await?;
            print_struct(&devices, cli.json)?;
        }
        Command::Show(args) => {
            let details = show_device(&paths, &args.device)?;
            print_struct(&details, cli.json)?;
        }
        Command::Track(args) => {
            let result = mutate_tracked_state(
                &paths,
                &args.device,
                TrackedState::Tracked,
                args.label.as_deref(),
                args.alias.as_deref(),
            )?;
            print_mutation(result, cli.json)?;
        }
        Command::Untrack(args) => {
            let result =
                mutate_tracked_state(&paths, &args.device, TrackedState::Untracked, None, None)?;
            print_mutation(result, cli.json)?;
        }
        Command::Ignore(args) => {
            let result =
                mutate_tracked_state(&paths, &args.device, TrackedState::Ignored, None, None)?;
            print_mutation(result, cli.json)?;
        }
        Command::Label(args) => {
            let store = open_store(&paths.db)?;
            let identity_id = lookup_or_exit(&store, &args.device)?;
            let result = store.set_label_by_id(&identity_id, &args.label)?;
            print_mutation(result, cli.json)?;
        }
        Command::Alias(args) => {
            let store = open_store(&paths.db)?;
            let identity_id = lookup_or_exit(&store, &args.device)?;
            let result = store.set_alias_by_id(&identity_id, &args.alias)?;
            print_mutation(result, cli.json)?;
        }
        Command::Discover {
            command: DiscoverCommand::List,
        } => {
            let devices = list_discovered(&cli, &paths).await?;
            print_struct(&devices, cli.json)?;
        }
        Command::Resolve(args) => {
            let resolved = resolve(&cli, &paths, &args.device, args.preference.as_core()).await?;
            if cli.json {
                print_struct(&resolved, true)?;
            } else if args.ssh_command {
                println!("ssh {}", resolved.ssh_args.join(" "));
            } else {
                ensure_resolved(&resolved)?;
                let destination = ssh_destination(&resolved);
                println!("{destination}");
            }
        }
        Command::Refresh(args) => {
            let mode = if args.full { "full" } else { "quick" };
            let mut client = connect_required(&cli, &paths).await?;
            let response = client
                .refresh(RefreshRequest {
                    mode: mode.to_string(),
                    device_query: args.device.clone().unwrap_or_default(),
                })
                .await?
                .into_inner();
            if cli.json {
                print_struct(
                    &serde_json::json!({ "accepted": response.accepted, "message": response.message }),
                    true,
                )?;
            } else {
                println!("{}", response.message);
            }
        }
        Command::Ssh(args) => {
            let resolved = resolve(&cli, &paths, &args.device, args.preference.as_core()).await?;
            ensure_resolved(&resolved)?;
            exec_ssh(&resolved.ssh_args, &args.ssh_args)?;
        }
    }

    Ok(())
}

#[derive(Debug)]
struct Paths {
    db: PathBuf,
    socket: PathBuf,
}

async fn daemon_status(cli: &Cli, paths: &Paths) -> Result<StatusOutput> {
    if !cli.offline {
        match network_manager_ipc::connect_uds(&paths.socket).await {
            Ok(mut client) => {
                let response = client
                    .get_daemon_status(GetDaemonStatusRequest {})
                    .await?
                    .into_inner();
                return Ok(StatusOutput {
                    state: response.state,
                    source: response.source,
                    db_path: empty_to_none(response.db_path),
                    started_at: empty_to_none(response.started_at),
                    updated_at: empty_to_none(response.updated_at),
                    stale: response.stale,
                    daemon_available: true,
                });
            }
            Err(error) if cli.require_daemon => {
                eprintln!("daemon unavailable: {error:#}");
                std::process::exit(EXIT_DAEMON_DOWN);
            }
            Err(_) => {}
        }
    }

    let store = open_store(&paths.db)?;
    let mut status = store.daemon_status("sqlite_fallback")?;
    status.stale = true;
    Ok(StatusOutput {
        state: status.state,
        source: status.source,
        db_path: status.db_path,
        started_at: status.started_at,
        updated_at: status.updated_at,
        stale: true,
        daemon_available: false,
    })
}

async fn list_devices(
    cli: &Cli,
    paths: &Paths,
    args: &DeviceListArgs,
) -> Result<Vec<DeviceOutput>> {
    if !cli.offline {
        match network_manager_ipc::connect_uds(&paths.socket).await {
            Ok(mut client) => {
                let response = client
                    .list_device_identities(ListDeviceIdentitiesRequest {
                        tracked_only: args.tracked,
                        ignored_only: args.ignored,
                    })
                    .await?
                    .into_inner();
                return Ok(response
                    .identities
                    .into_iter()
                    .map(|identity| DeviceOutput {
                        id: identity.id,
                        stable_key: identity.stable_key,
                        label: empty_to_none(identity.label),
                        alias: empty_to_none(identity.alias),
                        tracked_state: identity.tracked_state,
                        category: empty_to_none(identity.category),
                        tags: identity.tags,
                        ssh_username: empty_to_none(identity.ssh_username),
                        ssh_port: u16::try_from(identity.ssh_port)
                            .ok()
                            .filter(|port| *port != 0),
                        endpoint_preference: identity.endpoint_preference,
                        last_seen_at: empty_to_none(identity.last_seen_at),
                        endpoint_count: identity.endpoint_count as usize,
                    })
                    .collect());
            }
            Err(error) if cli.require_daemon => {
                eprintln!("daemon unavailable: {error:#}");
                std::process::exit(EXIT_DAEMON_DOWN);
            }
            Err(_) => {}
        }
    }

    let store = open_store(&paths.db)?;
    let mut records = store.list_device_identities()?;
    if args.tracked {
        records.retain(|record| record.identity.tracked_state.as_str() == "tracked");
    }
    if args.ignored {
        records.retain(|record| record.identity.tracked_state.as_str() == "ignored");
    }
    Ok(records
        .into_iter()
        .map(|record| DeviceOutput {
            id: record.identity.id,
            stable_key: record.identity.stable_key,
            label: record.identity.label,
            alias: record.identity.alias,
            tracked_state: record.identity.tracked_state.as_str().to_string(),
            category: record.identity.category,
            tags: record.identity.tags,
            ssh_username: record.identity.ssh_username,
            ssh_port: record.identity.ssh_port,
            endpoint_preference: record.identity.endpoint_preference.as_str().to_string(),
            last_seen_at: record.identity.last_seen_at,
            endpoint_count: record.endpoint_count,
        })
        .collect())
}

async fn list_discovered(cli: &Cli, paths: &Paths) -> Result<Vec<DiscoveredOutput>> {
    if !cli.offline {
        match network_manager_ipc::connect_uds(&paths.socket).await {
            Ok(mut client) => {
                let response = client
                    .list_discovered_devices(ListDiscoveredDevicesRequest {})
                    .await?
                    .into_inner();
                return Ok(response
                    .devices
                    .into_iter()
                    .map(|device| DiscoveredOutput {
                        id: device.id,
                        source: device.source,
                        source_device_id: device.source_device_id,
                        display_name: empty_to_none(device.display_name),
                        first_seen_at: device.first_seen_at,
                        last_seen_at: device.last_seen_at,
                        identity_id: empty_to_none(device.identity_id),
                    })
                    .collect());
            }
            Err(error) if cli.require_daemon => {
                eprintln!("daemon unavailable: {error:#}");
                std::process::exit(EXIT_DAEMON_DOWN);
            }
            Err(_) => {}
        }
    }

    let store = open_store(&paths.db)?;
    Ok(store
        .list_discovered_devices()?
        .into_iter()
        .map(|record| DiscoveredOutput {
            id: record.device.id,
            source: record.device.source,
            source_device_id: record.device.source_device_id,
            display_name: record.device.display_name,
            first_seen_at: record.device.first_seen_at,
            last_seen_at: record.device.last_seen_at,
            identity_id: record.identity_id,
        })
        .collect())
}

fn show_device(paths: &Paths, query: &str) -> Result<DeviceDetailsOutput> {
    let store = open_store(&paths.db)?;
    let identity_id = lookup_or_exit(&store, query)?;
    let Some(details) = store.device_details_by_id(&identity_id)? else {
        eprintln!("device '{query}' was not found");
        std::process::exit(EXIT_NOT_FOUND);
    };

    let endpoint_count = details.endpoints.len();
    Ok(DeviceDetailsOutput {
        device: device_output_from_identity(details.identity, endpoint_count),
        endpoints: details
            .endpoints
            .into_iter()
            .map(|endpoint| EndpointOutput {
                id: endpoint.id,
                kind: endpoint.kind.as_str().to_string(),
                address: endpoint.address,
                port: endpoint.port,
                hostname: endpoint.hostname,
                reachability: endpoint.reachability.as_str().to_string(),
                ssh_capability: endpoint.ssh_capability.as_str().to_string(),
                last_seen_at: endpoint.last_seen_at,
                last_checked_at: endpoint.last_checked_at,
            })
            .collect(),
    })
}

fn mutate_tracked_state(
    paths: &Paths,
    query: &str,
    state: TrackedState,
    label: Option<&str>,
    alias: Option<&str>,
) -> Result<DeviceMutationResult> {
    let store = open_store(&paths.db)?;
    let identity_id = lookup_or_exit(&store, query)?;
    store.set_tracked_state_by_id(&identity_id, state, label, alias)
}

fn lookup_or_exit(store: &SqliteStore, query: &str) -> Result<String> {
    match store.find_identity_id(query)? {
        IdentityLookup::Found(identity_id) => Ok(identity_id),
        IdentityLookup::NotFound => {
            eprintln!("device '{query}' was not found");
            std::process::exit(EXIT_NOT_FOUND);
        }
        IdentityLookup::Ambiguous(ids) => {
            eprintln!("device query '{query}' is ambiguous: {}", ids.join(", "));
            std::process::exit(EXIT_AMBIGUOUS);
        }
    }
}

fn print_mutation(result: DeviceMutationResult, json: bool) -> Result<()> {
    let output = MutationOutput {
        message: result.message,
        device: device_output_from_identity(result.identity, result.endpoint_count),
    };
    if json {
        print_struct(&output, true)
    } else {
        println!("{}", output.message);
        if let Some(alias) = output.device.alias {
            println!("alias: {alias}");
        }
        Ok(())
    }
}

fn device_output_from_identity(
    identity: network_manager_core::DeviceIdentity,
    endpoint_count: usize,
) -> DeviceOutput {
    DeviceOutput {
        id: identity.id,
        stable_key: identity.stable_key,
        label: identity.label,
        alias: identity.alias,
        tracked_state: identity.tracked_state.as_str().to_string(),
        category: identity.category,
        tags: identity.tags,
        ssh_username: identity.ssh_username,
        ssh_port: identity.ssh_port,
        endpoint_preference: identity.endpoint_preference.as_str().to_string(),
        last_seen_at: identity.last_seen_at,
        endpoint_count,
    }
}

async fn resolve(
    cli: &Cli,
    paths: &Paths,
    query: &str,
    preference: EndpointPreference,
) -> Result<ResolveOutput> {
    if !cli.offline {
        match network_manager_ipc::connect_uds(&paths.socket).await {
            Ok(mut client) => {
                let response = client
                    .resolve_ssh_target(ResolveSshTargetRequest {
                        device_query: query.to_string(),
                        endpoint_preference: preference.as_str().to_string(),
                    })
                    .await?
                    .into_inner();
                return Ok(ResolveOutput {
                    found: response.found,
                    ambiguous: response.ambiguous,
                    source: "daemon".to_string(),
                    candidate_identity_ids: response.candidate_identity_ids,
                    endpoint_id: empty_to_none(response.endpoint_id),
                    host: empty_to_none(response.host),
                    port: u16::try_from(response.port).ok().filter(|port| *port != 0),
                    username: empty_to_none(response.username),
                    endpoint_kind: empty_to_none(response.endpoint_kind),
                    ssh_args: response.ssh_args,
                    message: response.message,
                });
            }
            Err(error) if cli.require_daemon => {
                eprintln!("daemon unavailable: {error:#}");
                std::process::exit(EXIT_DAEMON_DOWN);
            }
            Err(_) => {}
        }
    }

    resolve_from_store(&paths.db, query, preference)
}

fn resolve_from_store(
    db_path: &PathBuf,
    query: &str,
    preference: EndpointPreference,
) -> Result<ResolveOutput> {
    let store = open_store(db_path)?;
    let identity_id = match store.find_identity_id(query)? {
        IdentityLookup::Found(identity_id) => identity_id,
        IdentityLookup::NotFound => {
            return Ok(ResolveOutput {
                found: false,
                ambiguous: false,
                source: "sqlite_fallback".to_string(),
                candidate_identity_ids: vec![],
                endpoint_id: None,
                host: None,
                port: None,
                username: None,
                endpoint_kind: None,
                ssh_args: vec![],
                message: format!("device '{query}' was not found"),
            })
        }
        IdentityLookup::Ambiguous(ids) => {
            return Ok(ResolveOutput {
                found: false,
                ambiguous: true,
                source: "sqlite_fallback".to_string(),
                candidate_identity_ids: ids,
                endpoint_id: None,
                host: None,
                port: None,
                username: None,
                endpoint_kind: None,
                ssh_args: vec![],
                message: format!("device query '{query}' is ambiguous"),
            })
        }
    };

    let identities = store.list_device_identities()?;
    let identity = identities
        .into_iter()
        .find(|record| record.identity.id == identity_id)
        .map(|record| record.identity);
    let endpoints = store.endpoints_for_identity(&identity_id)?;
    let username = identity
        .as_ref()
        .and_then(|identity| identity.ssh_username.as_deref());
    let ssh_port = identity.as_ref().and_then(|identity| identity.ssh_port);

    let Some(target) = resolve_ssh_target(&endpoints, preference, username, ssh_port) else {
        return Ok(ResolveOutput {
            found: false,
            ambiguous: false,
            source: "sqlite_fallback".to_string(),
            candidate_identity_ids: vec![identity_id],
            endpoint_id: None,
            host: None,
            port: None,
            username: None,
            endpoint_kind: None,
            ssh_args: vec![],
            message: "no available SSH target endpoint".to_string(),
        });
    };

    Ok(resolve_output_from_target(
        "sqlite_fallback",
        vec![identity_id],
        target,
    ))
}

fn resolve_output_from_target(
    source: &str,
    candidate_identity_ids: Vec<String>,
    target: SshTarget,
) -> ResolveOutput {
    let ssh_args = target.command_args();
    ResolveOutput {
        found: true,
        ambiguous: false,
        source: source.to_string(),
        candidate_identity_ids,
        endpoint_id: Some(target.endpoint_id),
        host: Some(target.host),
        port: Some(target.port),
        username: target.username,
        endpoint_kind: Some(target.endpoint_kind.as_str().to_string()),
        ssh_args,
        message: "resolved".to_string(),
    }
}

async fn connect_required(
    cli: &Cli,
    paths: &Paths,
) -> Result<
    network_manager_ipc::pb::network_manager_client::NetworkManagerClient<
        tonic::transport::Channel,
    >,
> {
    if cli.offline {
        std::process::exit(EXIT_DAEMON_DOWN);
    }
    network_manager_ipc::connect_uds(&paths.socket).await
}

fn open_store(path: &PathBuf) -> Result<SqliteStore> {
    let store = SqliteStore::open(path)?;
    store.migrate()?;
    Ok(store)
}

fn print_value(value: &str, json: bool) -> Result<()> {
    if json {
        print_struct(&serde_json::json!({ "value": value }), true)
    } else {
        println!("{value}");
        Ok(())
    }
}

fn print_struct<T: Serialize>(value: &T, _json: bool) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn empty_to_none(value: String) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn ensure_resolved(resolved: &ResolveOutput) -> Result<()> {
    if resolved.ambiguous {
        eprintln!("{}", resolved.message);
        std::process::exit(EXIT_AMBIGUOUS);
    }
    if !resolved.found {
        eprintln!("{}", resolved.message);
        let code = if resolved.candidate_identity_ids.is_empty() {
            EXIT_NOT_FOUND
        } else {
            EXIT_UNAVAILABLE
        };
        std::process::exit(code);
    }
    Ok(())
}

fn ssh_destination(resolved: &ResolveOutput) -> String {
    let host = resolved.host.clone().unwrap_or_default();
    match &resolved.username {
        Some(username) if !username.is_empty() => format!("{username}@{host}"),
        _ => host,
    }
}

#[cfg(unix)]
fn exec_ssh(base_args: &[String], extra_args: &[String]) -> Result<()> {
    use std::os::unix::process::CommandExt;
    let mut command = std::process::Command::new("ssh");
    command.args(base_args);
    command.args(extra_args);
    let error = command.exec();
    Err(anyhow!("failed to exec ssh: {error}"))
}

#[cfg(not(unix))]
fn exec_ssh(base_args: &[String], extra_args: &[String]) -> Result<()> {
    use anyhow::Context;
    let status = std::process::Command::new("ssh")
        .args(base_args)
        .args(extra_args)
        .status()
        .context("running ssh")?;
    std::process::exit(status.code().unwrap_or(1));
}
