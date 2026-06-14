use anyhow::{anyhow, Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use network_manager_core::{
    resolve_ssh_target, AvailabilityState, EndpointKind, EndpointPreference, SshTarget,
    TrackedState,
};
use network_manager_db::{
    DeviceMutationResult, IdentityCorrectionResult, IdentityLookup, SqliteStore, UserSettingsExport,
};
use network_manager_ipc::pb::{
    DeviceMutationResponse, DeviceTagRequest, GetDaemonStatusRequest, GetDeviceDetailsRequest,
    GetDeviceDetailsResponse, IdentityCorrectionResponse, ListDeviceIdentitiesRequest,
    ListDiscoveredDevicesRequest, MergeIdentitiesRequest, RefreshRequest, ResolveSshTargetRequest,
    SetDeviceCategoryRequest, SetDeviceTextRequest, SetEndpointPreferenceRequest,
    SetOptionalStringRequest, SetSshPortRequest, SetTrackedStateRequest,
    SplitDiscoveredDeviceRequest,
};
use serde::Serialize;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::str::FromStr;

const EXIT_NOT_FOUND: i32 = 20;
const EXIT_AMBIGUOUS: i32 = 21;
const EXIT_UNAVAILABLE: i32 = 22;
const EXIT_DAEMON_DOWN: i32 = 23;
const DEFAULT_LAUNCH_AGENT_LABEL: &str = "com.network-manager.daemon";

type IpcClient = network_manager_ipc::pb::network_manager_client::NetworkManagerClient<
    tonic::transport::Channel,
>;

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
    /// Set or clear the device category.
    Category(CategoryArgs),
    /// Add a tag to a device.
    Tag(TagArgs),
    /// Remove a tag from a device.
    Untag(TagArgs),
    /// Merge one identity into another.
    Merge(MergeArgs),
    /// Split one discovered device into a separate identity.
    Split(SplitArgs),
    /// Discovery commands.
    Discover {
        #[command(subcommand)]
        command: DiscoverCommand,
    },
    /// Resolve a device to an SSH target.
    Resolve(ResolveArgs),
    /// Poll device status until interrupted.
    Watch(WatchArgs),
    /// Request a bounded daemon refresh.
    Refresh(RefreshArgs),
    /// Emit local diagnostic counts and paths.
    Diagnostics(DiagnosticsArgs),
    /// Export portable user settings as JSON.
    Export(ExportArgs),
    /// Import portable user settings JSON.
    Import(ImportArgs),
    /// Print SSH config Host entries for resolved tracked aliases.
    SshConfig(SshConfigArgs),
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
    /// Print the LaunchAgent plist that would be installed.
    Plist(DaemonPlistArgs),
    /// Install a per-user LaunchAgent plist.
    Install(DaemonInstallArgs),
    /// Remove the per-user LaunchAgent plist.
    Uninstall(DaemonLabelArgs),
    /// Start the installed LaunchAgent.
    Start(DaemonLabelArgs),
    /// Restart the installed LaunchAgent.
    Restart(DaemonLabelArgs),
    /// Stop the installed LaunchAgent.
    Stop(DaemonLabelArgs),
}

#[derive(Debug, Args)]
struct DaemonPlistArgs {
    #[arg(long)]
    label: Option<String>,
    #[arg(long)]
    daemon_path: Option<PathBuf>,
    /// Seconds between automatic daemon quick refreshes.
    #[arg(long)]
    refresh_interval_seconds: Option<u64>,
    /// Disable daemon automatic refreshes in the generated plist.
    #[arg(long)]
    disable_auto_refresh: bool,
}

#[derive(Debug, Args)]
struct DaemonInstallArgs {
    #[arg(long)]
    label: Option<String>,
    #[arg(long)]
    daemon_path: Option<PathBuf>,
    /// Seconds between automatic daemon quick refreshes.
    #[arg(long)]
    refresh_interval_seconds: Option<u64>,
    /// Disable daemon automatic refreshes in the installed plist.
    #[arg(long)]
    disable_auto_refresh: bool,
    #[arg(long)]
    force: bool,
    #[arg(long)]
    load: bool,
}

#[derive(Debug, Args)]
struct DaemonLabelArgs {
    #[arg(long)]
    label: Option<String>,
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
    /// Set or clear the device category.
    Category(CategoryArgs),
    /// Add a tag to a device.
    Tag(TagArgs),
    /// Remove a tag from a device.
    Untag(TagArgs),
    /// Merge one identity into another.
    Merge(MergeArgs),
    /// Split one discovered device into a separate identity.
    Split(SplitArgs),
}

#[derive(Debug, Args)]
struct DeviceListArgs {
    #[arg(long)]
    tracked: bool,
    #[arg(long)]
    ignored: bool,
    /// Only devices with at least one online endpoint.
    #[arg(long)]
    online: bool,
    /// Only devices with at least one endpoint accepting SSH.
    #[arg(long)]
    ssh: bool,
    /// Filter by endpoint source/kind group: lan, tailscale, lan_ip, mdns, etc.
    #[arg(long)]
    source: Option<String>,
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
struct CategoryArgs {
    device: String,
    category: Option<String>,
    #[arg(long)]
    clear: bool,
}

#[derive(Debug, Args)]
struct TagArgs {
    device: String,
    tag: String,
}

#[derive(Debug, Args)]
struct MergeArgs {
    source: String,
    target: String,
    #[arg(long)]
    reason: Option<String>,
}

#[derive(Debug, Args)]
struct SplitArgs {
    /// Discovered device id from `network-manager discover list`.
    discovered_id: String,
    #[arg(long)]
    reason: Option<String>,
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
struct WatchArgs {
    /// Seconds between polls.
    #[arg(long, default_value_t = 5)]
    interval: u64,
    /// Run one poll and exit.
    #[arg(long)]
    once: bool,
    /// Ask the daemon for a quick refresh before each poll.
    #[arg(long)]
    refresh: bool,
    #[command(flatten)]
    filters: DeviceListArgs,
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
struct DiagnosticsArgs {
    /// Include raw local paths instead of redacted paths.
    #[arg(long)]
    no_redact: bool,
}

#[derive(Debug, Args)]
struct ExportArgs {
    /// Output path. Omit or pass '-' to write to stdout.
    path: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct ImportArgs {
    /// Input path. Omit or pass '-' to read from stdin.
    path: Option<PathBuf>,
    /// Validate and count what would be applied without writing.
    #[arg(long)]
    dry_run: bool,
}

#[derive(Debug, Args)]
struct SshConfigArgs {
    /// Include untracked devices with aliases too.
    #[arg(long)]
    all: bool,
    /// Output path. Omit or pass '-' to write to stdout.
    #[arg(long)]
    output: Option<PathBuf>,
    /// Overwrite an existing output file.
    #[arg(long)]
    force: bool,
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

impl DeviceListArgs {
    fn has_endpoint_filters(&self) -> bool {
        self.online || self.ssh || self.source.is_some()
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
struct CorrectionOutput {
    message: String,
    identity_id: String,
    affected_identity_id: String,
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

#[derive(Debug, Serialize)]
struct DiagnosticsOutput {
    daemon: StatusOutput,
    paths: DiagnosticsPaths,
    counts: DiagnosticsCounts,
}

#[derive(Debug, Serialize)]
struct DiagnosticsPaths {
    db: String,
    socket: String,
    launch_agent: String,
    logs: String,
}

#[derive(Debug, Serialize)]
struct DiagnosticsCounts {
    identities: usize,
    tracked: usize,
    ignored: usize,
    discovered: usize,
    endpoints: usize,
    online_endpoints: usize,
    ssh_endpoints: usize,
}

#[derive(Debug, Serialize)]
struct WatchOutput {
    observed_at: String,
    devices: Vec<DeviceOutput>,
}

#[derive(Debug, Serialize)]
struct SshConfigEntryOutput {
    host: String,
    hostname: String,
    port: u16,
    user: Option<String>,
    identity_id: String,
    endpoint_kind: String,
}

#[derive(Debug, Serialize)]
struct SshConfigOutput {
    entries: Vec<SshConfigEntryOutput>,
}

#[derive(Debug, Serialize)]
struct DaemonPlistOutput {
    label: String,
    daemon_path: String,
    db_path: String,
    socket_path: String,
    plist: String,
}

#[derive(Debug, Serialize)]
struct DaemonActionOutput {
    action: String,
    label: String,
    path: Option<String>,
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

    if cli.offline && cli.require_daemon {
        eprintln!("--offline conflicts with --require-daemon");
        std::process::exit(EXIT_DAEMON_DOWN);
    }

    match &cli.command {
        Command::Paths(args) => {
            match args.kind {
                PathKind::Db => print_value(&paths.db.display().to_string(), cli.json),
                PathKind::Socket => print_value(&paths.socket.display().to_string(), cli.json),
            }?;
        }
        Command::Daemon { command } => match command {
            DaemonCommand::Status => {
                let status = daemon_status(&cli, &paths).await?;
                print_struct(&status, cli.json)?;
            }
            DaemonCommand::Plist(args) => {
                let label = args.label.as_deref().unwrap_or(DEFAULT_LAUNCH_AGENT_LABEL);
                let daemon_path = canonical_daemon_path(
                    &args
                        .daemon_path
                        .clone()
                        .unwrap_or_else(default_daemon_binary_path),
                )?;
                let plist = launch_agent_plist(
                    label,
                    &daemon_path,
                    &paths,
                    args.refresh_interval_seconds,
                    args.disable_auto_refresh,
                )?;
                let output = DaemonPlistOutput {
                    label: label.to_string(),
                    daemon_path: daemon_path.display().to_string(),
                    db_path: paths.db.display().to_string(),
                    socket_path: paths.socket.display().to_string(),
                    plist,
                };
                if cli.json {
                    print_struct(&output, true)?;
                } else {
                    println!("{}", output.plist);
                }
            }
            DaemonCommand::Install(args) => {
                let label = args.label.as_deref().unwrap_or(DEFAULT_LAUNCH_AGENT_LABEL);
                let daemon_path = canonical_daemon_path(
                    &args
                        .daemon_path
                        .clone()
                        .unwrap_or_else(default_daemon_binary_path),
                )?;
                let plist_path = install_launch_agent(
                    label,
                    &daemon_path,
                    &paths,
                    args.refresh_interval_seconds,
                    args.disable_auto_refresh,
                    args.force,
                )?;
                let mut message = format!("installed {}", plist_path.display());
                if args.load {
                    launchctl_bootstrap(label)?;
                    message = format!("{message}; started {label}");
                }
                print_daemon_action("install", label, Some(&plist_path), &message, cli.json)?;
            }
            DaemonCommand::Uninstall(args) => {
                let label = args.label.as_deref().unwrap_or(DEFAULT_LAUNCH_AGENT_LABEL);
                validate_launch_agent_label(label)?;
                launchctl_bootout(label).ok();
                let path = launch_agent_path(label);
                let message = if path.exists() {
                    std::fs::remove_file(&path)?;
                    format!("removed {}", path.display())
                } else {
                    format!("{} was not installed", path.display())
                };
                print_daemon_action("uninstall", label, Some(&path), &message, cli.json)?;
            }
            DaemonCommand::Start(args) => {
                let label = args.label.as_deref().unwrap_or(DEFAULT_LAUNCH_AGENT_LABEL);
                launchctl_bootstrap(label)?;
                print_daemon_action(
                    "start",
                    label,
                    Some(&launch_agent_path(label)),
                    &format!("started {label}"),
                    cli.json,
                )?;
            }
            DaemonCommand::Restart(args) => {
                let label = args.label.as_deref().unwrap_or(DEFAULT_LAUNCH_AGENT_LABEL);
                restart_launch_agent(label)?;
                print_daemon_action(
                    "restart",
                    label,
                    Some(&launch_agent_path(label)),
                    &format!("restarted {label}"),
                    cli.json,
                )?;
            }
            DaemonCommand::Stop(args) => {
                let label = args.label.as_deref().unwrap_or(DEFAULT_LAUNCH_AGENT_LABEL);
                launchctl_bootout(label)?;
                print_daemon_action(
                    "stop",
                    label,
                    Some(&launch_agent_path(label)),
                    &format!("stopped {label}"),
                    cli.json,
                )?;
            }
        },
        Command::Devices { command } => match command {
            DevicesCommand::List(args) => {
                let devices = list_devices(&cli, &paths, args).await?;
                print_struct(&devices, cli.json)?;
            }
            DevicesCommand::Show(args) => {
                let details = show_device(&cli, &paths, &args.device).await?;
                print_struct(&details, cli.json)?;
            }
            DevicesCommand::Track(args) => {
                let result = mutate_tracked_state(
                    &cli,
                    &paths,
                    &args.device,
                    TrackedState::Tracked,
                    args.label.as_deref(),
                    args.alias.as_deref(),
                )
                .await?;
                print_mutation(result, cli.json)?;
            }
            DevicesCommand::Untrack(args) => {
                let result = mutate_tracked_state(
                    &cli,
                    &paths,
                    &args.device,
                    TrackedState::Untracked,
                    None,
                    None,
                )
                .await?;
                print_mutation(result, cli.json)?;
            }
            DevicesCommand::Ignore(args) => {
                let result = mutate_tracked_state(
                    &cli,
                    &paths,
                    &args.device,
                    TrackedState::Ignored,
                    None,
                    None,
                )
                .await?;
                print_mutation(result, cli.json)?;
            }
            DevicesCommand::Label(args) => {
                let result = set_label(&cli, &paths, &args.device, &args.label).await?;
                print_mutation(result, cli.json)?;
            }
            DevicesCommand::Alias(args) => {
                let result = set_alias(&cli, &paths, &args.device, &args.alias).await?;
                print_mutation(result, cli.json)?;
            }
            DevicesCommand::Category(args) => {
                let result = set_category(
                    &cli,
                    &paths,
                    &args.device,
                    args.category.as_deref(),
                    args.clear,
                )
                .await?;
                print_mutation(result, cli.json)?;
            }
            DevicesCommand::Tag(args) => {
                let result = add_tag(&cli, &paths, &args.device, &args.tag).await?;
                print_mutation(result, cli.json)?;
            }
            DevicesCommand::Untag(args) => {
                let result = remove_tag(&cli, &paths, &args.device, &args.tag).await?;
                print_mutation(result, cli.json)?;
            }
            DevicesCommand::SshUser(args) => {
                let result = set_ssh_username(
                    &cli,
                    &paths,
                    &args.device,
                    args.username.as_deref(),
                    args.clear,
                )
                .await?;
                print_mutation(result, cli.json)?;
            }
            DevicesCommand::SshPort(args) => {
                let result =
                    set_ssh_port(&cli, &paths, &args.device, args.port, args.clear).await?;
                print_mutation(result, cli.json)?;
            }
            DevicesCommand::Preference(args) => {
                let result =
                    set_endpoint_preference(&cli, &paths, &args.device, args.preference.as_core())
                        .await?;
                print_mutation(result, cli.json)?;
            }
            DevicesCommand::Merge(args) => {
                let result = merge_identities(
                    &cli,
                    &paths,
                    &args.source,
                    &args.target,
                    args.reason.as_deref(),
                )
                .await?;
                print_correction(result, cli.json)?;
            }
            DevicesCommand::Split(args) => {
                let result =
                    split_discovered(&cli, &paths, &args.discovered_id, args.reason.as_deref())
                        .await?;
                print_correction(result, cli.json)?;
            }
        },
        Command::List(args) => {
            let devices = list_devices(&cli, &paths, args).await?;
            print_struct(&devices, cli.json)?;
        }
        Command::Show(args) => {
            let details = show_device(&cli, &paths, &args.device).await?;
            print_struct(&details, cli.json)?;
        }
        Command::Track(args) => {
            let result = mutate_tracked_state(
                &cli,
                &paths,
                &args.device,
                TrackedState::Tracked,
                args.label.as_deref(),
                args.alias.as_deref(),
            )
            .await?;
            print_mutation(result, cli.json)?;
        }
        Command::Untrack(args) => {
            let result = mutate_tracked_state(
                &cli,
                &paths,
                &args.device,
                TrackedState::Untracked,
                None,
                None,
            )
            .await?;
            print_mutation(result, cli.json)?;
        }
        Command::Ignore(args) => {
            let result = mutate_tracked_state(
                &cli,
                &paths,
                &args.device,
                TrackedState::Ignored,
                None,
                None,
            )
            .await?;
            print_mutation(result, cli.json)?;
        }
        Command::Label(args) => {
            let result = set_label(&cli, &paths, &args.device, &args.label).await?;
            print_mutation(result, cli.json)?;
        }
        Command::Alias(args) => {
            let result = set_alias(&cli, &paths, &args.device, &args.alias).await?;
            print_mutation(result, cli.json)?;
        }
        Command::Category(args) => {
            let result = set_category(
                &cli,
                &paths,
                &args.device,
                args.category.as_deref(),
                args.clear,
            )
            .await?;
            print_mutation(result, cli.json)?;
        }
        Command::Tag(args) => {
            let result = add_tag(&cli, &paths, &args.device, &args.tag).await?;
            print_mutation(result, cli.json)?;
        }
        Command::Untag(args) => {
            let result = remove_tag(&cli, &paths, &args.device, &args.tag).await?;
            print_mutation(result, cli.json)?;
        }
        Command::Merge(args) => {
            let result = merge_identities(
                &cli,
                &paths,
                &args.source,
                &args.target,
                args.reason.as_deref(),
            )
            .await?;
            print_correction(result, cli.json)?;
        }
        Command::Split(args) => {
            let result =
                split_discovered(&cli, &paths, &args.discovered_id, args.reason.as_deref()).await?;
            print_correction(result, cli.json)?;
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
                ensure_resolved(&resolved)?;
                println!("ssh {}", resolved.ssh_args.join(" "));
            } else {
                ensure_resolved(&resolved)?;
                let destination = ssh_destination(&resolved);
                println!("{destination}");
            }
        }
        Command::Watch(args) => {
            watch_devices(&cli, &paths, args).await?;
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
        Command::Diagnostics(args) => {
            let diagnostics = diagnostics(&cli, &paths, !args.no_redact).await?;
            print_struct(&diagnostics, cli.json)?;
        }
        Command::Export(args) => {
            export_settings(&paths, args.path.as_deref())?;
        }
        Command::Import(args) => {
            let result = import_settings(&paths, args.path.as_deref(), args.dry_run)?;
            print_struct(&result, cli.json)?;
        }
        Command::SshConfig(args) => {
            ssh_config(
                &paths,
                args.all,
                args.output.as_deref(),
                args.force,
                cli.json,
            )?;
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

fn default_daemon_binary_path() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|path| {
            path.parent()
                .map(|parent| parent.join("network-manager-daemon"))
        })
        .unwrap_or_else(|| PathBuf::from("network-manager-daemon"))
}

fn launch_agent_path(label: &str) -> PathBuf {
    home_dir()
        .join("Library")
        .join("LaunchAgents")
        .join(format!("{label}.plist"))
}

fn validate_launch_agent_label(label: &str) -> Result<()> {
    if label.is_empty() {
        return Err(anyhow!("LaunchAgent label cannot be empty"));
    }
    if label.contains('/') || label.contains('\\') || label.contains("..") {
        return Err(anyhow!(
            "LaunchAgent label contains an unsafe path component"
        ));
    }
    if !label
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
    {
        return Err(anyhow!(
            "LaunchAgent label may contain only ASCII letters, digits, '.', '_' and '-'"
        ));
    }
    Ok(())
}

fn canonical_daemon_path(path: &Path) -> Result<PathBuf> {
    let canonical = path
        .canonicalize()
        .with_context(|| format!("resolving daemon binary path {}", path.display()))?;
    if !canonical.is_file() {
        return Err(anyhow!(
            "daemon binary is not a file: {}",
            canonical.display()
        ));
    }
    Ok(canonical)
}

fn print_daemon_action(
    action: &str,
    label: &str,
    path: Option<&Path>,
    message: &str,
    json: bool,
) -> Result<()> {
    let output = DaemonActionOutput {
        action: action.to_string(),
        label: label.to_string(),
        path: path.map(|path| path.display().to_string()),
        message: message.to_string(),
    };
    if json {
        print_struct(&output, true)
    } else {
        println!("{}", output.message);
        Ok(())
    }
}

fn log_dir() -> PathBuf {
    home_dir()
        .join("Library")
        .join("Logs")
        .join("Network Manager")
}

fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn launch_agent_plist(
    label: &str,
    daemon_path: &Path,
    paths: &Paths,
    refresh_interval_seconds: Option<u64>,
    disable_auto_refresh: bool,
) -> Result<String> {
    validate_launch_agent_label(label)?;
    if !daemon_path.is_absolute() {
        return Err(anyhow!("daemon binary path must be absolute"));
    }
    let logs = log_dir();
    let stdout = logs.join("daemon.log");
    let stderr = logs.join("daemon.err.log");
    let refresh_args = launch_agent_refresh_args(refresh_interval_seconds, disable_auto_refresh);
    Ok(format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>{label}</string>
  <key>EnvironmentVariables</key>
  <dict>
    <key>PATH</key>
    <string>/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin</string>
  </dict>
  <key>ProgramArguments</key>
  <array>
    <string>{daemon}</string>
    <string>--db</string>
    <string>{db}</string>
    <string>--socket</string>
    <string>{socket}</string>
{refresh_args}  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>StandardOutPath</key>
  <string>{stdout}</string>
  <key>StandardErrorPath</key>
  <string>{stderr}</string>
</dict>
</plist>
"#,
        label = xml_escape(label),
        daemon = xml_escape(&daemon_path.display().to_string()),
        db = xml_escape(&paths.db.display().to_string()),
        socket = xml_escape(&paths.socket.display().to_string()),
        refresh_args = refresh_args,
        stdout = xml_escape(&stdout.display().to_string()),
        stderr = xml_escape(&stderr.display().to_string()),
    ))
}

fn launch_agent_refresh_args(
    refresh_interval_seconds: Option<u64>,
    disable_auto_refresh: bool,
) -> String {
    let mut output = String::new();
    if let Some(seconds) = refresh_interval_seconds {
        output.push_str("    <string>--refresh-interval-seconds</string>\n");
        output.push_str(&format!("    <string>{seconds}</string>\n"));
    }
    if disable_auto_refresh {
        output.push_str("    <string>--disable-auto-refresh</string>\n");
    }
    output
}

fn install_launch_agent(
    label: &str,
    daemon_path: &Path,
    paths: &Paths,
    refresh_interval_seconds: Option<u64>,
    disable_auto_refresh: bool,
    force: bool,
) -> Result<PathBuf> {
    validate_launch_agent_label(label)?;
    if !daemon_path.is_absolute() || !daemon_path.is_file() {
        return Err(anyhow!(
            "daemon binary must be an absolute file path: {}",
            daemon_path.display()
        ));
    }
    let plist_path = launch_agent_path(label);
    if plist_path.exists() && !force {
        return Err(anyhow!(
            "LaunchAgent already exists at {}; use --force to overwrite",
            plist_path.display()
        ));
    }
    if let Some(parent) = plist_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::create_dir_all(log_dir())?;
    if let Some(parent) = paths.db.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if let Some(parent) = paths.socket.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let plist = launch_agent_plist(
        label,
        daemon_path,
        paths,
        refresh_interval_seconds,
        disable_auto_refresh,
    )?;
    std::fs::write(&plist_path, plist)?;
    Ok(plist_path)
}

fn launchctl_bootstrap(label: &str) -> Result<()> {
    validate_launch_agent_label(label)?;
    let plist_path = launch_agent_path(label);
    if !plist_path.exists() {
        return Err(anyhow!(
            "LaunchAgent is not installed: {}",
            plist_path.display()
        ));
    }
    run_launchctl(&[
        "bootstrap".to_string(),
        gui_domain()?,
        plist_path.display().to_string(),
    ])
}

fn launchctl_bootout(label: &str) -> Result<()> {
    validate_launch_agent_label(label)?;
    let service = format!("{}/{}", gui_domain()?, label);
    run_launchctl(&["bootout".to_string(), service])
}

fn restart_launch_agent(label: &str) -> Result<()> {
    match launchctl_bootout(label) {
        Ok(()) => {}
        Err(error) if launchctl_missing_process_error(&error) => {}
        Err(error) => return Err(error).context("stopping LaunchAgent before restart"),
    }
    launchctl_bootstrap(label).context("starting LaunchAgent after restart")
}

fn launchctl_missing_process_error(error: &anyhow::Error) -> bool {
    let message = error.to_string();
    message.contains("No such process") || message.contains("service is not loaded")
}

fn run_launchctl(args: &[String]) -> Result<()> {
    let output = std::process::Command::new("launchctl")
        .args(args)
        .output()?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            Err(anyhow!("launchctl exited with status {}", output.status))
        } else {
            Err(anyhow!(
                "launchctl exited with status {}: {stderr}",
                output.status
            ))
        }
    }
}

fn gui_domain() -> Result<String> {
    let output = std::process::Command::new("id").arg("-u").output()?;
    if !output.status.success() {
        return Err(anyhow!("id -u failed"));
    }
    let uid = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(format!("gui/{uid}"))
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
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

async fn diagnostics(cli: &Cli, paths: &Paths, redact: bool) -> Result<DiagnosticsOutput> {
    let daemon = daemon_status(cli, paths).await?;
    let store = open_store(&paths.db)?;
    let identities = store.list_device_identities()?;
    let discovered = store.list_discovered_devices()?;

    let mut endpoints = 0;
    let mut online_endpoints = 0;
    let mut ssh_endpoints = 0;
    for identity in &identities {
        for endpoint in store.endpoints_for_identity(&identity.identity.id)? {
            endpoints += 1;
            if endpoint.reachability == AvailabilityState::Online {
                online_endpoints += 1;
            }
            if endpoint.ssh_capability == AvailabilityState::Online {
                ssh_endpoints += 1;
            }
        }
    }

    let paths_output = DiagnosticsPaths {
        db: format_diagnostic_path(&paths.db, redact),
        socket: format_diagnostic_path(&paths.socket, redact),
        launch_agent: format_diagnostic_path(
            &launch_agent_path(DEFAULT_LAUNCH_AGENT_LABEL),
            redact,
        ),
        logs: format_diagnostic_path(&log_dir(), redact),
    };

    Ok(DiagnosticsOutput {
        daemon,
        paths: paths_output,
        counts: DiagnosticsCounts {
            tracked: identities
                .iter()
                .filter(|identity| identity.identity.tracked_state == TrackedState::Tracked)
                .count(),
            ignored: identities
                .iter()
                .filter(|identity| identity.identity.tracked_state == TrackedState::Ignored)
                .count(),
            identities: identities.len(),
            discovered: discovered.len(),
            endpoints,
            online_endpoints,
            ssh_endpoints,
        },
    })
}

fn format_diagnostic_path(path: &Path, redact: bool) -> String {
    if !redact {
        return path.display().to_string();
    }
    let home = home_dir();
    match path.strip_prefix(&home) {
        Ok(relative) => PathBuf::from("~").join(relative).display().to_string(),
        Err(_) => path.display().to_string(),
    }
}

async fn watch_devices(cli: &Cli, paths: &Paths, args: &WatchArgs) -> Result<()> {
    if args.interval == 0 && !args.once {
        return Err(anyhow!(
            "--interval must be greater than 0 unless --once is used"
        ));
    }

    loop {
        if args.refresh && !cli.offline {
            refresh_quick_for_watch(cli, paths).await?;
        }
        let output = WatchOutput {
            observed_at: unix_timestamp_string(),
            devices: list_devices(cli, paths, &args.filters).await?,
        };
        if cli.json {
            println!("{}", serde_json::to_string(&output)?);
        } else {
            print_watch_output(&output);
        }

        if args.once {
            break;
        }

        tokio::select! {
            _ = tokio::signal::ctrl_c() => break,
            _ = tokio::time::sleep(std::time::Duration::from_secs(args.interval)) => {}
        }
    }

    Ok(())
}

async fn refresh_quick_for_watch(cli: &Cli, paths: &Paths) -> Result<()> {
    match network_manager_ipc::connect_uds(&paths.socket).await {
        Ok(mut client) => {
            client
                .refresh(RefreshRequest {
                    mode: "quick".to_string(),
                    device_query: String::new(),
                })
                .await?;
        }
        Err(error) if cli.require_daemon => {
            eprintln!("daemon unavailable: {error:#}");
            std::process::exit(EXIT_DAEMON_DOWN);
        }
        Err(error) => eprintln!("watch refresh skipped; daemon unavailable: {error:#}"),
    }
    Ok(())
}

fn print_watch_output(output: &WatchOutput) {
    println!(
        "[{}] {} device(s)",
        output.observed_at,
        output.devices.len()
    );
    for device in &output.devices {
        let name = device
            .alias
            .as_deref()
            .or(device.label.as_deref())
            .unwrap_or(&device.stable_key);
        let tags = if device.tags.is_empty() {
            String::new()
        } else {
            format!(" tags={}", device.tags.join(","))
        };
        let category = device
            .category
            .as_deref()
            .map(|category| format!(" category={category}"))
            .unwrap_or_default();
        println!(
            "  {name} id={} state={} endpoints={}{}{}",
            device.id, device.tracked_state, device.endpoint_count, category, tags
        );
    }
}

fn unix_timestamp_string() -> String {
    match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => duration.as_secs().to_string(),
        Err(_) => "0".to_string(),
    }
}

fn export_settings(paths: &Paths, path: Option<&Path>) -> Result<()> {
    let store = open_store(&paths.db)?;
    let document = store.export_user_settings()?;
    let text = serde_json::to_string_pretty(&document)?;
    match path.filter(|path| path.as_os_str() != "-") {
        Some(path) => {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, format!("{text}\n"))?;
        }
        None => println!("{text}"),
    }
    Ok(())
}

fn import_settings(
    paths: &Paths,
    path: Option<&Path>,
    dry_run: bool,
) -> Result<network_manager_db::UserSettingsImportResult> {
    let mut text = String::new();
    match path.filter(|path| path.as_os_str() != "-") {
        Some(path) => text = std::fs::read_to_string(path)?,
        None => {
            std::io::stdin().read_to_string(&mut text)?;
        }
    }
    let document: UserSettingsExport = serde_json::from_str(&text)?;
    let store = open_store(&paths.db)?;
    store.import_user_settings(&document, dry_run)
}

fn ssh_config(
    paths: &Paths,
    include_all: bool,
    output: Option<&Path>,
    force: bool,
    json: bool,
) -> Result<()> {
    let entries = ssh_config_entries(paths, include_all)?;
    if json {
        return print_struct(&SshConfigOutput { entries }, true);
    }

    let text = render_ssh_config(&entries);
    match output.filter(|path| path.as_os_str() != "-") {
        Some(path) => {
            if path.exists() && !force {
                return Err(anyhow!(
                    "{} already exists; use --force to overwrite",
                    path.display()
                ));
            }
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, text)?;
        }
        None => print!("{text}"),
    }
    Ok(())
}

fn ssh_config_entries(paths: &Paths, include_all: bool) -> Result<Vec<SshConfigEntryOutput>> {
    let store = open_store(&paths.db)?;
    let mut entries = Vec::new();
    for record in store.list_device_identities()? {
        let identity = record.identity;
        if !include_all && identity.tracked_state != TrackedState::Tracked {
            continue;
        }
        let Some(alias) = identity.alias.as_deref().filter(|alias| !alias.is_empty()) else {
            continue;
        };
        let endpoints = store.endpoints_for_identity(&identity.id)?;
        let Some(target) = resolve_ssh_target(
            &endpoints,
            identity.endpoint_preference,
            identity.ssh_username.as_deref(),
            identity.ssh_port,
        ) else {
            continue;
        };
        validate_ssh_config_token("Host", alias)?;
        validate_ssh_config_token("HostName", &target.host)?;
        if let Some(username) = target.username.as_deref() {
            validate_ssh_config_token("User", username)?;
        }
        entries.push(SshConfigEntryOutput {
            host: alias.to_string(),
            hostname: target.host,
            port: target.port,
            user: target.username,
            identity_id: identity.id,
            endpoint_kind: target.endpoint_kind.as_str().to_string(),
        });
    }
    entries.sort_by(|left, right| left.host.cmp(&right.host));
    Ok(entries)
}

fn validate_ssh_config_token(field: &str, value: &str) -> Result<()> {
    if value.is_empty()
        || value
            .chars()
            .any(|ch| ch.is_control() || ch.is_ascii_whitespace())
    {
        return Err(anyhow!(
            "cannot render invalid SSH config {field} value: {value:?}"
        ));
    }
    Ok(())
}

fn render_ssh_config(entries: &[SshConfigEntryOutput]) -> String {
    let mut text = String::new();
    text.push_str("# Generated by network-manager ssh-config. Review before including.\n");
    for entry in entries {
        text.push_str(&format!("\nHost {}\n", entry.host));
        text.push_str(&format!("  HostName {}\n", entry.hostname));
        text.push_str(&format!("  Port {}\n", entry.port));
        if let Some(user) = entry.user.as_deref().filter(|user| !user.is_empty()) {
            text.push_str(&format!("  User {user}\n"));
        }
        text.push_str(&format!(
            "  # network-manager identity={} endpoint_kind={}\n",
            entry.identity_id, entry.endpoint_kind
        ));
    }
    text
}

async fn list_devices(
    cli: &Cli,
    paths: &Paths,
    args: &DeviceListArgs,
) -> Result<Vec<DeviceOutput>> {
    if !cli.offline && args.has_endpoint_filters() && cli.require_daemon {
        if let Err(error) = network_manager_ipc::connect_uds(&paths.socket).await {
            eprintln!("daemon unavailable: {error:#}");
            std::process::exit(EXIT_DAEMON_DOWN);
        }
    }

    if !cli.offline && !args.has_endpoint_filters() {
        match network_manager_ipc::connect_uds(&paths.socket).await {
            Ok(mut client) => {
                let response = client
                    .list_device_identities(ListDeviceIdentitiesRequest {
                        tracked_only: args.tracked,
                        ignored_only: args.ignored,
                    })
                    .await?
                    .into_inner();
                return response
                    .identities
                    .into_iter()
                    .map(device_output_from_ipc)
                    .collect::<Result<Vec<_>>>();
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
    if args.has_endpoint_filters() {
        let mut filtered = Vec::new();
        for record in records {
            if device_matches_endpoint_filters(&store, &record.identity.id, args)? {
                filtered.push(record);
            }
        }
        records = filtered;
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

fn device_matches_endpoint_filters(
    store: &SqliteStore,
    identity_id: &str,
    args: &DeviceListArgs,
) -> Result<bool> {
    let endpoints = store.endpoints_for_identity(identity_id)?;
    if args.online
        && !endpoints
            .iter()
            .any(|endpoint| endpoint.reachability == AvailabilityState::Online)
    {
        return Ok(false);
    }
    if args.ssh
        && !endpoints
            .iter()
            .any(|endpoint| endpoint.ssh_capability == AvailabilityState::Online)
    {
        return Ok(false);
    }
    if let Some(source) = args.source.as_deref() {
        let source = validate_endpoint_source_filter(source)?;
        if !endpoints
            .iter()
            .any(|endpoint| endpoint_kind_matches_source(endpoint.kind, &source))
        {
            return Ok(false);
        }
    }
    Ok(true)
}

fn validate_endpoint_source_filter(source: &str) -> Result<String> {
    let source = source.to_ascii_lowercase();
    match source.as_str() {
        "arp" | "lan" | "ts" | "tailscale" | "lan_dns" | "mdns" | "lan_ip"
        | "tailscale_dns" | "tailscale_ip" | "other" => Ok(source),
        _ => Err(anyhow!(
            "unknown endpoint source '{source}' (expected lan, arp, mdns, tailscale, lan_ip, lan_dns, tailscale_ip, tailscale_dns, or other)"
        )),
    }
}

fn endpoint_kind_matches_source(kind: EndpointKind, source: &str) -> bool {
    match source {
        "arp" => matches!(kind, EndpointKind::LanDns | EndpointKind::LanIp),
        "lan" => kind.is_lan(),
        "ts" | "tailscale" => kind.is_tailscale(),
        other => kind.as_str() == other,
    }
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

async fn show_device(cli: &Cli, paths: &Paths, query: &str) -> Result<DeviceDetailsOutput> {
    if !cli.offline {
        match network_manager_ipc::connect_uds(&paths.socket).await {
            Ok(mut client) => {
                let response = client
                    .get_device_details(GetDeviceDetailsRequest {
                        device_query: query.to_string(),
                    })
                    .await?
                    .into_inner();
                return device_details_from_response(response);
            }
            Err(error) if cli.require_daemon => {
                eprintln!("daemon unavailable: {error:#}");
                std::process::exit(EXIT_DAEMON_DOWN);
            }
            Err(_) => {}
        }
    }
    show_device_from_store(paths, query)
}

async fn mutate_tracked_state(
    cli: &Cli,
    paths: &Paths,
    query: &str,
    state: TrackedState,
    label: Option<&str>,
    alias: Option<&str>,
) -> Result<DeviceMutationResult> {
    if !cli.offline {
        match network_manager_ipc::connect_uds(&paths.socket).await {
            Ok(mut client) => {
                let response = client
                    .set_tracked_state(SetTrackedStateRequest {
                        device_query: query.to_string(),
                        tracked_state: state.as_str().to_string(),
                        label: label.unwrap_or_default().to_string(),
                        alias: alias.unwrap_or_default().to_string(),
                    })
                    .await?
                    .into_inner();
                return mutation_from_response(response);
            }
            Err(error) if cli.require_daemon => {
                eprintln!("daemon unavailable: {error:#}");
                std::process::exit(EXIT_DAEMON_DOWN);
            }
            Err(_) => {}
        }
    }
    mutate_tracked_state_from_store(paths, query, state, label, alias)
}

async fn set_label(
    cli: &Cli,
    paths: &Paths,
    query: &str,
    label: &str,
) -> Result<DeviceMutationResult> {
    if !cli.offline {
        match network_manager_ipc::connect_uds(&paths.socket).await {
            Ok(mut client) => {
                let response = client
                    .set_device_label(SetDeviceTextRequest {
                        device_query: query.to_string(),
                        value: label.to_string(),
                    })
                    .await?
                    .into_inner();
                return mutation_from_response(response);
            }
            Err(error) if cli.require_daemon => {
                eprintln!("daemon unavailable: {error:#}");
                std::process::exit(EXIT_DAEMON_DOWN);
            }
            Err(_) => {}
        }
    }
    set_label_from_store(paths, query, label)
}

async fn set_alias(
    cli: &Cli,
    paths: &Paths,
    query: &str,
    alias: &str,
) -> Result<DeviceMutationResult> {
    if !cli.offline {
        match network_manager_ipc::connect_uds(&paths.socket).await {
            Ok(mut client) => {
                let response = client
                    .set_device_alias(SetDeviceTextRequest {
                        device_query: query.to_string(),
                        value: alias.to_string(),
                    })
                    .await?
                    .into_inner();
                return mutation_from_response(response);
            }
            Err(error) if cli.require_daemon => {
                eprintln!("daemon unavailable: {error:#}");
                std::process::exit(EXIT_DAEMON_DOWN);
            }
            Err(_) => {}
        }
    }
    set_alias_from_store(paths, query, alias)
}

async fn set_category(
    cli: &Cli,
    paths: &Paths,
    query: &str,
    category: Option<&str>,
    clear: bool,
) -> Result<DeviceMutationResult> {
    let category = if clear {
        None
    } else {
        Some(category.ok_or_else(|| anyhow!("category is required unless --clear is used"))?)
    };

    if !cli.offline {
        match network_manager_ipc::connect_uds(&paths.socket).await {
            Ok(mut client) => {
                let response = client
                    .set_device_category(SetDeviceCategoryRequest {
                        device_query: query.to_string(),
                        category: category.unwrap_or_default().to_string(),
                        clear,
                    })
                    .await?
                    .into_inner();
                return mutation_from_response(response);
            }
            Err(error) if cli.require_daemon => {
                eprintln!("daemon unavailable: {error:#}");
                std::process::exit(EXIT_DAEMON_DOWN);
            }
            Err(_) => {}
        }
    }
    set_category_from_store(paths, query, category, clear)
}

async fn add_tag(cli: &Cli, paths: &Paths, query: &str, tag: &str) -> Result<DeviceMutationResult> {
    if !cli.offline {
        match network_manager_ipc::connect_uds(&paths.socket).await {
            Ok(mut client) => {
                let response = client
                    .add_device_tag(DeviceTagRequest {
                        device_query: query.to_string(),
                        tag: tag.to_string(),
                    })
                    .await?
                    .into_inner();
                return mutation_from_response(response);
            }
            Err(error) if cli.require_daemon => {
                eprintln!("daemon unavailable: {error:#}");
                std::process::exit(EXIT_DAEMON_DOWN);
            }
            Err(_) => {}
        }
    }
    add_tag_from_store(paths, query, tag)
}

async fn remove_tag(
    cli: &Cli,
    paths: &Paths,
    query: &str,
    tag: &str,
) -> Result<DeviceMutationResult> {
    if !cli.offline {
        match network_manager_ipc::connect_uds(&paths.socket).await {
            Ok(mut client) => {
                let response = client
                    .remove_device_tag(DeviceTagRequest {
                        device_query: query.to_string(),
                        tag: tag.to_string(),
                    })
                    .await?
                    .into_inner();
                return mutation_from_response(response);
            }
            Err(error) if cli.require_daemon => {
                eprintln!("daemon unavailable: {error:#}");
                std::process::exit(EXIT_DAEMON_DOWN);
            }
            Err(_) => {}
        }
    }
    remove_tag_from_store(paths, query, tag)
}

async fn set_ssh_username(
    cli: &Cli,
    paths: &Paths,
    query: &str,
    username: Option<&str>,
    clear: bool,
) -> Result<DeviceMutationResult> {
    if !clear && username.is_none() {
        return Err(anyhow!("SSH username is required unless --clear is used"));
    }
    if !cli.offline {
        match network_manager_ipc::connect_uds(&paths.socket).await {
            Ok(mut client) => {
                let response = client
                    .set_ssh_username(SetOptionalStringRequest {
                        device_query: query.to_string(),
                        value: username.unwrap_or_default().to_string(),
                        clear,
                    })
                    .await?
                    .into_inner();
                return mutation_from_response(response);
            }
            Err(error) if cli.require_daemon => {
                eprintln!("daemon unavailable: {error:#}");
                std::process::exit(EXIT_DAEMON_DOWN);
            }
            Err(_) => {}
        }
    }
    set_ssh_username_from_store(paths, query, username, clear)
}

async fn set_ssh_port(
    cli: &Cli,
    paths: &Paths,
    query: &str,
    port: Option<u16>,
    clear: bool,
) -> Result<DeviceMutationResult> {
    if !clear && port.is_none() {
        return Err(anyhow!("SSH port is required unless --clear is used"));
    }
    if !cli.offline {
        match network_manager_ipc::connect_uds(&paths.socket).await {
            Ok(mut client) => {
                let response = client
                    .set_ssh_port(SetSshPortRequest {
                        device_query: query.to_string(),
                        port: port.unwrap_or_default() as u32,
                        clear,
                    })
                    .await?
                    .into_inner();
                return mutation_from_response(response);
            }
            Err(error) if cli.require_daemon => {
                eprintln!("daemon unavailable: {error:#}");
                std::process::exit(EXIT_DAEMON_DOWN);
            }
            Err(_) => {}
        }
    }
    set_ssh_port_from_store(paths, query, port, clear)
}

async fn set_endpoint_preference(
    cli: &Cli,
    paths: &Paths,
    query: &str,
    preference: EndpointPreference,
) -> Result<DeviceMutationResult> {
    if !cli.offline {
        match network_manager_ipc::connect_uds(&paths.socket).await {
            Ok(mut client) => {
                let response = client
                    .set_endpoint_preference(SetEndpointPreferenceRequest {
                        device_query: query.to_string(),
                        endpoint_preference: preference.as_str().to_string(),
                    })
                    .await?
                    .into_inner();
                return mutation_from_response(response);
            }
            Err(error) if cli.require_daemon => {
                eprintln!("daemon unavailable: {error:#}");
                std::process::exit(EXIT_DAEMON_DOWN);
            }
            Err(_) => {}
        }
    }
    set_endpoint_preference_from_store(paths, query, preference)
}

async fn merge_identities(
    cli: &Cli,
    paths: &Paths,
    source_query: &str,
    target_query: &str,
    reason: Option<&str>,
) -> Result<IdentityCorrectionResult> {
    if !cli.offline {
        match network_manager_ipc::connect_uds(&paths.socket).await {
            Ok(mut client) => {
                let response = client
                    .merge_identities(MergeIdentitiesRequest {
                        source_query: source_query.to_string(),
                        target_query: target_query.to_string(),
                        reason: reason.unwrap_or_default().to_string(),
                    })
                    .await?
                    .into_inner();
                return correction_from_response(response);
            }
            Err(error) if cli.require_daemon => {
                eprintln!("daemon unavailable: {error:#}");
                std::process::exit(EXIT_DAEMON_DOWN);
            }
            Err(_) => {}
        }
    }
    merge_identities_from_store(paths, source_query, target_query, reason)
}

async fn split_discovered(
    cli: &Cli,
    paths: &Paths,
    discovered_id: &str,
    reason: Option<&str>,
) -> Result<IdentityCorrectionResult> {
    if !cli.offline {
        match network_manager_ipc::connect_uds(&paths.socket).await {
            Ok(mut client) => {
                let response = client
                    .split_discovered_device(SplitDiscoveredDeviceRequest {
                        discovered_device_id: discovered_id.to_string(),
                        reason: reason.unwrap_or_default().to_string(),
                    })
                    .await?
                    .into_inner();
                return correction_from_response(response);
            }
            Err(error) if cli.require_daemon => {
                eprintln!("daemon unavailable: {error:#}");
                std::process::exit(EXIT_DAEMON_DOWN);
            }
            Err(_) => {}
        }
    }
    split_discovered_from_store(paths, discovered_id, reason)
}

fn show_device_from_store(paths: &Paths, query: &str) -> Result<DeviceDetailsOutput> {
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

fn mutate_tracked_state_from_store(
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

fn set_label_from_store(paths: &Paths, query: &str, label: &str) -> Result<DeviceMutationResult> {
    let store = open_store(&paths.db)?;
    let identity_id = lookup_or_exit(&store, query)?;
    store.set_label_by_id(&identity_id, label)
}

fn set_alias_from_store(paths: &Paths, query: &str, alias: &str) -> Result<DeviceMutationResult> {
    let store = open_store(&paths.db)?;
    let identity_id = lookup_or_exit(&store, query)?;
    store.set_alias_by_id(&identity_id, alias)
}

fn set_category_from_store(
    paths: &Paths,
    query: &str,
    category: Option<&str>,
    clear: bool,
) -> Result<DeviceMutationResult> {
    let store = open_store(&paths.db)?;
    let identity_id = lookup_or_exit(&store, query)?;
    let category = if clear {
        None
    } else {
        Some(category.ok_or_else(|| anyhow!("category is required unless --clear is used"))?)
    };
    store.set_category_by_id(&identity_id, category)
}

fn add_tag_from_store(paths: &Paths, query: &str, tag: &str) -> Result<DeviceMutationResult> {
    let store = open_store(&paths.db)?;
    let identity_id = lookup_or_exit(&store, query)?;
    store.add_tag_by_id(&identity_id, tag)
}

fn remove_tag_from_store(paths: &Paths, query: &str, tag: &str) -> Result<DeviceMutationResult> {
    let store = open_store(&paths.db)?;
    let identity_id = lookup_or_exit(&store, query)?;
    store.remove_tag_by_id(&identity_id, tag)
}

fn set_ssh_username_from_store(
    paths: &Paths,
    query: &str,
    username: Option<&str>,
    clear: bool,
) -> Result<DeviceMutationResult> {
    let store = open_store(&paths.db)?;
    let identity_id = lookup_or_exit(&store, query)?;
    let username = if clear { None } else { username };
    store.set_ssh_username_by_id(&identity_id, username)
}

fn set_ssh_port_from_store(
    paths: &Paths,
    query: &str,
    port: Option<u16>,
    clear: bool,
) -> Result<DeviceMutationResult> {
    let store = open_store(&paths.db)?;
    let identity_id = lookup_or_exit(&store, query)?;
    let port = if clear { None } else { port };
    store.set_ssh_port_by_id(&identity_id, port)
}

fn set_endpoint_preference_from_store(
    paths: &Paths,
    query: &str,
    preference: EndpointPreference,
) -> Result<DeviceMutationResult> {
    let store = open_store(&paths.db)?;
    let identity_id = lookup_or_exit(&store, query)?;
    store.set_endpoint_preference_by_id(&identity_id, preference)
}

fn merge_identities_from_store(
    paths: &Paths,
    source_query: &str,
    target_query: &str,
    reason: Option<&str>,
) -> Result<IdentityCorrectionResult> {
    let store = open_store(&paths.db)?;
    let source_id = lookup_or_exit(&store, source_query)?;
    let target_id = lookup_or_exit(&store, target_query)?;
    store.merge_identities_by_id(&source_id, &target_id, reason)
}

fn split_discovered_from_store(
    paths: &Paths,
    discovered_id: &str,
    reason: Option<&str>,
) -> Result<IdentityCorrectionResult> {
    let store = open_store(&paths.db)?;
    store.split_discovered_device_by_id(discovered_id, reason)
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

fn device_details_from_response(response: GetDeviceDetailsResponse) -> Result<DeviceDetailsOutput> {
    if response.ambiguous {
        eprintln!(
            "{}: {}",
            response.message,
            response.candidate_identity_ids.join(", ")
        );
        std::process::exit(EXIT_AMBIGUOUS);
    }
    if !response.found {
        eprintln!("{}", response.message);
        std::process::exit(EXIT_NOT_FOUND);
    }
    let device = response
        .device
        .context("daemon returned a found details response without a device")?;
    Ok(DeviceDetailsOutput {
        device: device_output_from_ipc(device)?,
        endpoints: response
            .endpoints
            .into_iter()
            .map(endpoint_output_from_ipc)
            .collect::<Result<Vec<_>>>()?,
    })
}

fn mutation_from_response(response: DeviceMutationResponse) -> Result<DeviceMutationResult> {
    if response.ambiguous {
        eprintln!(
            "{}: {}",
            response.message,
            response.candidate_identity_ids.join(", ")
        );
        std::process::exit(EXIT_AMBIGUOUS);
    }
    if !response.found {
        eprintln!("{}", response.message);
        std::process::exit(EXIT_NOT_FOUND);
    }
    let device = response
        .device
        .context("daemon returned a mutation response without a device")?;
    let endpoint_count = device.endpoint_count as usize;
    Ok(DeviceMutationResult {
        identity: identity_from_ipc(device)?,
        endpoint_count,
        message: response.message,
    })
}

fn correction_from_response(
    response: IdentityCorrectionResponse,
) -> Result<IdentityCorrectionResult> {
    if response.ambiguous {
        eprintln!(
            "{}: {}",
            response.message,
            response.candidate_identity_ids.join(", ")
        );
        std::process::exit(EXIT_AMBIGUOUS);
    }
    if !response.applied {
        eprintln!("{}", response.message);
        std::process::exit(EXIT_NOT_FOUND);
    }
    Ok(IdentityCorrectionResult {
        identity_id: response.identity_id,
        affected_identity_id: response.affected_identity_id,
        message: response.message,
    })
}

fn identity_from_ipc(
    identity: network_manager_ipc::pb::DeviceIdentity,
) -> Result<network_manager_core::DeviceIdentity> {
    let tracked_state = TrackedState::from_str(&identity.tracked_state).with_context(|| {
        format!(
            "daemon returned invalid tracked_state '{}'",
            identity.tracked_state
        )
    })?;
    let endpoint_preference = EndpointPreference::from_str(&identity.endpoint_preference)
        .with_context(|| {
            format!(
                "daemon returned invalid endpoint_preference '{}'",
                identity.endpoint_preference
            )
        })?;

    Ok(network_manager_core::DeviceIdentity {
        id: identity.id,
        stable_key: identity.stable_key,
        label: empty_to_none(identity.label),
        alias: empty_to_none(identity.alias),
        tracked_state,
        category: empty_to_none(identity.category),
        tags: identity.tags,
        ssh_username: empty_to_none(identity.ssh_username),
        ssh_port: optional_ipc_port("ssh_port", identity.ssh_port)?,
        endpoint_preference,
        last_seen_at: empty_to_none(identity.last_seen_at),
    })
}

fn device_output_from_ipc(
    identity: network_manager_ipc::pb::DeviceIdentity,
) -> Result<DeviceOutput> {
    let endpoint_count = identity.endpoint_count as usize;
    Ok(device_output_from_identity(
        identity_from_ipc(identity)?,
        endpoint_count,
    ))
}

fn endpoint_output_from_ipc(
    endpoint: network_manager_ipc::pb::NetworkEndpoint,
) -> Result<EndpointOutput> {
    Ok(EndpointOutput {
        id: endpoint.id,
        kind: endpoint.kind,
        address: endpoint.address,
        port: optional_ipc_port("endpoint.port", endpoint.port)?,
        hostname: empty_to_none(endpoint.hostname),
        reachability: endpoint.reachability,
        ssh_capability: endpoint.ssh_capability,
        last_seen_at: empty_to_none(endpoint.last_seen_at),
        last_checked_at: empty_to_none(endpoint.last_checked_at),
    })
}

fn optional_ipc_port(field_name: &str, value: u32) -> Result<Option<u16>> {
    if value == 0 {
        return Ok(None);
    }

    u16::try_from(value)
        .map(Some)
        .with_context(|| format!("daemon returned invalid {field_name} {value}"))
}

fn print_correction(result: IdentityCorrectionResult, json: bool) -> Result<()> {
    let output = CorrectionOutput {
        message: result.message,
        identity_id: result.identity_id,
        affected_identity_id: result.affected_identity_id,
    };
    if json {
        print_struct(&output, true)
    } else {
        println!("{}", output.message);
        println!("identity: {}", output.identity_id);
        println!("affected: {}", output.affected_identity_id);
        Ok(())
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
                    port: optional_ipc_port("resolve port", response.port)?,
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

async fn connect_required(cli: &Cli, paths: &Paths) -> Result<IpcClient> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daemon_restart_command_parses() {
        let cli = Cli::try_parse_from(["network-manager", "daemon", "restart"]).unwrap();

        let Command::Daemon {
            command: DaemonCommand::Restart(args),
        } = cli.command
        else {
            panic!("expected daemon restart command");
        };
        assert_eq!(args.label, None);
    }

    #[test]
    fn missing_launch_agent_process_is_restartable() {
        let error = anyhow!(
            "launchctl exited with status exit status: 3: Boot-out failed: 3: No such process"
        );

        assert!(launchctl_missing_process_error(&error));
    }

    #[test]
    fn unrelated_launchctl_error_is_not_restartable() {
        let error = anyhow!("launchctl exited with status exit status: 5: Input/output error");

        assert!(!launchctl_missing_process_error(&error));
    }
}
