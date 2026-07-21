use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use network_manager_core::TrackedState;
use network_manager_ipc::pb::{RefreshRequest, SetTrackedStateRequest};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshMode {
    Quick,
    Full,
}

impl RefreshMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Quick => "quick",
            Self::Full => "full",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionOutcome {
    pub message: String,
    pub detail: Option<String>,
}

impl ActionOutcome {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            detail: None,
        }
    }

    pub fn combine(first: Self, second: Self) -> Self {
        let message = format!("{} {}", sentence(&first.message), second.message);
        let detail = match (first.detail, second.detail) {
            (Some(first_detail), Some(second_detail)) => {
                Some(format!("{first_detail}\n{second_detail}"))
            }
            (Some(first_detail), None) => Some(first_detail),
            (None, Some(second_detail)) => Some(second_detail),
            (None, None) => None,
        };
        Self { message, detail }
    }
}

fn sentence(value: &str) -> String {
    if value.ends_with(['.', '!', '?']) {
        value.to_string()
    } else {
        format!("{value}.")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonLifecycleAction {
    InstallAndStart,
    Start,
    Restart,
    Stop,
}

impl DaemonLifecycleAction {
    pub fn label(self) -> &'static str {
        match self {
            Self::InstallAndStart => "install daemon",
            Self::Start => "start daemon",
            Self::Restart => "restart daemon",
            Self::Stop => "stop daemon",
        }
    }
}

pub trait NetworkManagerActions: Send + Sync {
    fn refresh(&self, mode: RefreshMode) -> Result<ActionOutcome>;
    fn ensure_backend(&self) -> Result<ActionOutcome>;
    fn set_tracked_state(&self, device_query: &str, state: TrackedState) -> Result<ActionOutcome>;
    fn daemon_lifecycle(&self, action: DaemonLifecycleAction) -> Result<ActionOutcome>;
    fn open_diagnostics_folder(&self) -> Result<ActionOutcome>;
}

#[derive(Debug, Clone)]
pub struct DaemonActions {
    socket_path: PathBuf,
}

impl DaemonActions {
    pub fn new(socket_path: impl AsRef<Path>) -> Self {
        Self {
            socket_path: socket_path.as_ref().to_path_buf(),
        }
    }

    fn run<F, T>(&self, future: F) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>>,
    {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(future)
    }

    fn helper_binary_path(name: &str) -> Result<PathBuf> {
        let exe = std::env::current_exe()?;
        let exe_dir = exe
            .parent()
            .ok_or_else(|| anyhow::anyhow!("current executable has no parent directory"))?;
        let candidates = [
            exe_dir.join(name),
            exe_dir.join("..").join("Resources").join(name),
        ];
        candidates
            .into_iter()
            .find(|path| path.is_file())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "could not find helper binary '{name}' next to the UI; build or package network-manager and network-manager-daemon first"
                )
            })
    }

    fn daemon_available(&self) -> bool {
        let socket = self.socket_path.clone();
        self.run(async move {
            let mut client = network_manager_ipc::connect_uds(&socket).await?;
            client
                .get_daemon_status(network_manager_ipc::pb::GetDaemonStatusRequest {})
                .await?;
            Ok(())
        })
        .is_ok()
    }

    fn log_dir() -> PathBuf {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Library")
            .join("Logs")
            .join("Network Manager")
    }

    fn spawn_sidecar_daemon(&self) -> Result<()> {
        let daemon_path = Self::helper_binary_path("network-manager-daemon")?;
        let log_dir = Self::log_dir();
        std::fs::create_dir_all(&log_dir)
            .with_context(|| format!("creating log directory {}", log_dir.display()))?;
        let stdout = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_dir.join("daemon.sidecar.log"))?;
        let stderr = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_dir.join("daemon.sidecar.err.log"))?;
        Command::new(daemon_path)
            .arg("--db")
            .arg(network_manager_db::default_db_path())
            .arg("--socket")
            .arg(&self.socket_path)
            .stdin(Stdio::null())
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .spawn()
            .context("starting bundled network-manager-daemon")?;
        Ok(())
    }

    fn run_cli(args: &[String]) -> Result<String> {
        let output = Command::new(Self::helper_binary_path("network-manager")?)
            .args(args)
            .output()?;
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if !output.status.success() {
            anyhow::bail!(
                "network-manager {} failed: {}{}{}",
                args.join(" "),
                stdout,
                if stdout.is_empty() || stderr.is_empty() {
                    ""
                } else {
                    "\n"
                },
                stderr
            );
        }
        Ok(if stdout.is_empty() { stderr } else { stdout })
    }
}

impl Default for DaemonActions {
    fn default() -> Self {
        Self::new(network_manager_ipc::default_socket_path())
    }
}

impl NetworkManagerActions for DaemonActions {
    fn refresh(&self, mode: RefreshMode) -> Result<ActionOutcome> {
        let socket = self.socket_path.clone();
        self.run(async move {
            let mut client = network_manager_ipc::connect_uds(&socket).await?;
            let response = client
                .refresh(RefreshRequest {
                    mode: mode.as_str().to_string(),
                    device_query: String::new(),
                })
                .await?
                .into_inner();
            if !response.accepted {
                bail!(response.message);
            }
            Ok(ActionOutcome::new(response.message))
        })
    }

    fn ensure_backend(&self) -> Result<ActionOutcome> {
        if self.daemon_available() {
            return Ok(ActionOutcome::new("Local daemon is already running."));
        }

        self.spawn_sidecar_daemon()?;
        for _ in 0..40 {
            if self.daemon_available() {
                return Ok(ActionOutcome::new("Local daemon started."));
            }
            thread::sleep(Duration::from_millis(150));
        }

        bail!(
            "bundled daemon did not become ready on socket {}",
            self.socket_path.display()
        )
    }

    fn set_tracked_state(&self, device_query: &str, state: TrackedState) -> Result<ActionOutcome> {
        let socket = self.socket_path.clone();
        let device_query = device_query.to_string();
        self.run(async move {
            let mut client = network_manager_ipc::connect_uds(&socket).await?;
            let response = client
                .set_tracked_state(SetTrackedStateRequest {
                    device_query,
                    tracked_state: state.as_str().to_string(),
                    label: String::new(),
                    alias: String::new(),
                })
                .await?
                .into_inner();
            if response.ambiguous {
                bail!(
                    "device query is ambiguous: {}",
                    response.candidate_identity_ids.join(", ")
                );
            }
            if !response.found {
                bail!(response.message);
            }
            Ok(ActionOutcome::new(response.message))
        })
    }

    fn daemon_lifecycle(&self, action: DaemonLifecycleAction) -> Result<ActionOutcome> {
        let mut args = Vec::new();
        args.push("daemon".to_string());
        match action {
            DaemonLifecycleAction::InstallAndStart => {
                let daemon_path = Self::helper_binary_path("network-manager-daemon")?;
                args.push("install".to_string());
                args.push("--daemon-path".to_string());
                args.push(daemon_path.display().to_string());
                args.push("--force".to_string());
                args.push("--load".to_string());
            }
            DaemonLifecycleAction::Start => args.push("start".to_string()),
            DaemonLifecycleAction::Stop => args.push("stop".to_string()),
            DaemonLifecycleAction::Restart => args.push("restart".to_string()),
        }
        let message = Self::run_cli(&args)?;
        Ok(if message.is_empty() {
            ActionOutcome::new(format!("{} complete.", action.label()))
        } else {
            ActionOutcome::new(message)
        })
    }

    fn open_diagnostics_folder(&self) -> Result<ActionOutcome> {
        let log_dir = Self::log_dir();
        std::fs::create_dir_all(&log_dir)
            .with_context(|| format!("creating diagnostics folder {}", log_dir.display()))?;
        let output = Command::new("open")
            .arg(&log_dir)
            .output()
            .with_context(|| format!("opening diagnostics folder {}", log_dir.display()))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if stderr.is_empty() {
                bail!("open exited with status {}", output.status);
            }
            bail!("open diagnostics folder failed: {stderr}");
        }
        Ok(ActionOutcome::new(format!(
            "Opened diagnostics folder {}.",
            log_dir.display()
        )))
    }
}
