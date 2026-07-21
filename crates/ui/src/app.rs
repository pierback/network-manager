use std::sync::Arc;

use gpui::{
    prelude::*, px, size, AppContext, Bounds, ClipboardItem, Context, FocusHandle, Render, Window,
    WindowBounds, WindowOptions,
};
use network_manager_core::TrackedState;

use crate::data::{
    ActionOutcome, ActionStatus, DaemonActions, DaemonLifecycleAction, NetworkManagerActions,
    NetworkManagerRepository, RefreshMode, SqliteRepository,
};
use crate::hotkeys::{
    BringAllToFront, CloseWindow, MinimizeWindow, NewWindow, NextRoute, PreviousRoute, RefreshFull,
    RefreshQuick, ShowDashboard, ShowDeviceDetail, ShowDiscovery, ShowKeyboardShortcuts,
    ShowQuickAccess, ShowSettings, ToggleFullscreen, ZoomWindow, KEY_CONTEXT,
};
use crate::layout::app_shell::window_shell;
use crate::routes::{DiscoveryFilter, Route};
use crate::screens::{dashboard, device_detail, discovery, quick_access, settings};
use crate::theme::LiquidGlassTokens;

const ACTION_STATUS_SUMMARY_CHAR_LIMIT: usize = 110;

pub struct NetworkManagerApp {
    route: Route,
    repository: Box<dyn NetworkManagerRepository>,
    actions: Arc<dyn NetworkManagerActions>,
    selected_device_id: Option<String>,
    selected_discovery_filter: DiscoveryFilter,
    startup_backend_checked: bool,
    action_status: Option<ActionStatus>,
    action_sequence: u64,
    tokens: LiquidGlassTokens,
    focus_handle: Option<FocusHandle>,
}

impl NetworkManagerApp {
    pub fn live() -> Self {
        Self::new_with_actions(SqliteRepository::default(), DaemonActions::default())
    }

    fn new_with_actions(
        repository: impl NetworkManagerRepository + 'static,
        actions: impl NetworkManagerActions + 'static,
    ) -> Self {
        Self {
            route: Route::Dashboard,
            repository: Box::new(repository),
            actions: Arc::new(actions),
            selected_device_id: None,
            selected_discovery_filter: DiscoveryFilter::AllSources,
            startup_backend_checked: false,
            action_status: None,
            action_sequence: 0,
            tokens: LiquidGlassTokens::default(),
            focus_handle: None,
        }
    }

    #[cfg(test)]
    fn current_route(&self) -> Route {
        self.route
    }

    #[cfg(test)]
    fn action_status(&self) -> Option<&ActionStatus> {
        self.action_status.as_ref()
    }

    #[cfg(test)]
    fn current_discovery_filter(&self) -> DiscoveryFilter {
        self.selected_discovery_filter
    }

    #[cfg(test)]
    fn selected_device_id(&self) -> Option<&str> {
        self.selected_device_id.as_deref()
    }

    pub(crate) fn set_route(&mut self, route: Route, cx: &mut Context<Self>) {
        self.route = route;
        cx.notify();
    }

    pub(crate) fn select_device_detail(&mut self, identity_id: String, cx: &mut Context<Self>) {
        self.selected_device_id = Some(identity_id);
        self.route = Route::DeviceDetail;
        cx.notify();
    }

    pub(crate) fn select_discovery_filter(
        &mut self,
        filter: DiscoveryFilter,
        cx: &mut Context<Self>,
    ) {
        self.selected_discovery_filter = filter;
        self.route = Route::Discovery;
        cx.notify();
    }

    #[cfg(test)]
    fn select_route_for_test(&mut self, route: Route) {
        self.route = route;
    }

    #[cfg(test)]
    fn refresh_for_test(&mut self, mode: RefreshMode) {
        let result = self.actions.refresh(mode);
        self.record_action_result(result);
    }

    #[cfg(test)]
    fn daemon_lifecycle_for_test(&mut self, action: DaemonLifecycleAction) {
        let result = self.actions.daemon_lifecycle(action);
        self.record_action_result(result);
    }

    #[cfg(test)]
    fn open_diagnostics_folder_for_test(&mut self) {
        let result = self.actions.open_diagnostics_folder();
        self.record_action_result(result);
    }

    #[cfg(test)]
    fn set_discovery_identity_state_for_test(
        &mut self,
        identity_id: Option<String>,
        state: TrackedState,
    ) {
        self.set_discovery_identity_state_inner(identity_id, state);
    }

    pub(crate) fn refresh_quick(&mut self, cx: &mut Context<Self>) {
        self.start_refresh(RefreshMode::Quick, cx);
    }

    pub(crate) fn refresh_full(&mut self, cx: &mut Context<Self>) {
        self.start_refresh(RefreshMode::Full, cx);
    }

    pub(crate) fn set_discovery_identity_state(
        &mut self,
        identity_id: Option<String>,
        state: TrackedState,
        cx: &mut Context<Self>,
    ) {
        if self.record_busy_action(cx) {
            return;
        }
        let Some(identity_id) = identity_id else {
            self.invalidate_pending_action();
            self.action_status = Some(ActionStatus::failed(
                "Cannot update this discovery yet because it has no device identity.",
                None,
            ));
            cx.notify();
            return;
        };
        let action = state.as_str().to_string();
        self.start_action(
            format!("Marking {identity_id} {action}…"),
            cx,
            move |actions| actions.set_tracked_state(&identity_id, state),
        );
    }

    pub(crate) fn install_and_start_daemon(&mut self, cx: &mut Context<Self>) {
        self.start_daemon_lifecycle(DaemonLifecycleAction::InstallAndStart, cx);
    }

    pub(crate) fn start_daemon(&mut self, cx: &mut Context<Self>) {
        self.start_daemon_lifecycle(DaemonLifecycleAction::Start, cx);
    }

    pub(crate) fn restart_daemon(&mut self, cx: &mut Context<Self>) {
        self.start_daemon_lifecycle(DaemonLifecycleAction::Restart, cx);
    }

    pub(crate) fn stop_daemon(&mut self, cx: &mut Context<Self>) {
        self.start_daemon_lifecycle(DaemonLifecycleAction::Stop, cx);
    }

    pub(crate) fn open_diagnostics_folder(&mut self, cx: &mut Context<Self>) {
        self.start_action("Opening diagnostics folder…".into(), cx, move |actions| {
            actions.open_diagnostics_folder()
        });
    }

    fn start_daemon_lifecycle(&mut self, action: DaemonLifecycleAction, cx: &mut Context<Self>) {
        self.start_action(
            format!("{} running…", action.label()),
            cx,
            move |actions| actions.daemon_lifecycle(action),
        );
    }

    fn start_startup_backend(&mut self, cx: &mut Context<Self>) {
        self.start_action(
            "Starting local daemon and refreshing…".into(),
            cx,
            move |actions| {
                let backend = actions.ensure_backend()?;
                let refresh = actions.refresh(RefreshMode::Quick)?;
                Ok(ActionOutcome::combine(backend, refresh))
            },
        );
    }

    fn start_refresh(&mut self, mode: RefreshMode, cx: &mut Context<Self>) {
        self.start_action(
            format!("{} refresh running…", mode.as_str()),
            cx,
            move |actions| actions.refresh(mode),
        );
    }

    fn open_new_window(&mut self, cx: &mut Context<Self>) {
        let bounds = Bounds::centered(None, size(px(1280.0), px(800.0)), cx);
        let options = WindowOptions {
            titlebar: None,
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            window_min_size: Some(size(px(640.0), px(480.0))),
            ..Default::default()
        };
        if let Err(error) = cx.open_window(options, |_, cx| cx.new(|_| NetworkManagerApp::live())) {
            self.action_status = Some(ActionStatus::failed(
                format!("Could not open a new window: {error}"),
                None,
            ));
            cx.notify();
        }
    }

    fn start_action(
        &mut self,
        pending_message: String,
        cx: &mut Context<Self>,
        job: impl FnOnce(Arc<dyn NetworkManagerActions>) -> anyhow::Result<ActionOutcome>
            + Send
            + 'static,
    ) {
        if self.record_busy_action(cx) {
            return;
        }
        self.action_status = Some(ActionStatus::pending(pending_message));
        let action_sequence = self.next_action_sequence();
        cx.notify();

        let actions = self.actions.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { job(actions) })
                .await;
            this.update(cx, |app, cx| {
                app.record_action_result_if_current(action_sequence, result);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    #[cfg(test)]
    fn set_discovery_identity_state_inner(
        &mut self,
        identity_id: Option<String>,
        state: TrackedState,
    ) {
        let Some(identity_id) = identity_id else {
            self.invalidate_pending_action();
            self.action_status = Some(ActionStatus::failed(
                "Cannot update this discovery yet because it has no device identity.",
                None,
            ));
            return;
        };
        let result = self.actions.set_tracked_state(&identity_id, state);
        self.record_action_result(result);
    }

    fn record_busy_action(&mut self, cx: &mut Context<Self>) -> bool {
        if self
            .action_status
            .as_ref()
            .is_some_and(ActionStatus::is_pending)
        {
            self.action_status = Some(ActionStatus::pending(
                "Another network action is already running; wait for it to finish.",
            ));
            cx.notify();
            true
        } else {
            false
        }
    }

    fn next_action_sequence(&mut self) -> u64 {
        self.action_sequence = self.action_sequence.wrapping_add(1);
        self.action_sequence
    }

    fn invalidate_pending_action(&mut self) {
        self.next_action_sequence();
    }

    fn record_action_result_if_current(
        &mut self,
        action_sequence: u64,
        result: anyhow::Result<ActionOutcome>,
    ) {
        if action_sequence == self.action_sequence {
            self.record_action_result(result);
        }
    }

    fn record_action_result(&mut self, result: anyhow::Result<ActionOutcome>) {
        self.action_status = Some(match result {
            Ok(outcome) => action_status_from_outcome(outcome, false),
            Err(error) => {
                action_status_from_outcome(ActionOutcome::new(format!("{error:#}")), true)
            }
        });
    }

    fn previous_route(&mut self, cx: &mut Context<Self>) {
        let index = Route::ALL
            .iter()
            .position(|route| *route == self.route)
            .unwrap_or(0);
        let previous = if index == 0 {
            Route::ALL[Route::ALL.len() - 1]
        } else {
            Route::ALL[index - 1]
        };
        self.set_route(previous, cx);
    }

    fn next_route(&mut self, cx: &mut Context<Self>) {
        let index = Route::ALL
            .iter()
            .position(|route| *route == self.route)
            .unwrap_or(0);
        let next = Route::ALL[(index + 1) % Route::ALL.len()];
        self.set_route(next, cx);
    }

    pub(crate) fn show_keyboard_shortcuts(&mut self, cx: &mut Context<Self>) {
        self.action_status = Some(ActionStatus::succeeded(
            "⌘1 Dashboard · ⌘2 Discovery · ⌘3 Detail · ⌘4 Quick Access · ⌘, Settings · ⌘R Refresh · ⇧⌘R Full Refresh · ⌘[/⌘] Previous/Next · ⌘K Quick Access · ⌘/ Shortcuts",
            None,
        ));
        cx.notify();
    }

    pub(crate) fn copy_selected_ssh_command(&mut self, cx: &mut Context<Self>) {
        let detail = self
            .repository
            .selected_device_detail(self.selected_device_id.as_deref());
        let Some(target) = detail.preferred_target else {
            self.record_detail_copy_error(
                "No SSH target is available for the selected device.",
                cx,
            );
            return;
        };
        let command = target.command;
        cx.write_to_clipboard(ClipboardItem::new_string(command.clone()));
        self.record_info_message(format!("Copied {command}"), cx);
    }

    pub(crate) fn copy_selected_target(&mut self, cx: &mut Context<Self>) {
        let detail = self
            .repository
            .selected_device_detail(self.selected_device_id.as_deref());
        let Some(target) = detail.preferred_target else {
            self.record_detail_copy_error("No target is available for the selected device.", cx);
            return;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(target.destination.clone()));
        self.record_info_message(format!("Copied {}", target.destination), cx);
    }

    fn record_detail_copy_error(&mut self, message: &'static str, cx: &mut Context<Self>) {
        self.action_status = Some(ActionStatus::failed(message, None));
        cx.notify();
    }

    pub(crate) fn record_info_message(
        &mut self,
        message: impl Into<String>,
        cx: &mut Context<Self>,
    ) {
        self.action_status = Some(ActionStatus::succeeded(message, None));
        cx.notify();
    }

    fn ensure_focus_handle(&mut self, window: &mut Window, cx: &mut Context<Self>) -> FocusHandle {
        if let Some(handle) = &self.focus_handle {
            return handle.clone();
        }
        let handle = cx.focus_handle();
        handle.focus(window);
        self.focus_handle = Some(handle.clone());
        handle
    }
}

fn action_status_from_outcome(outcome: ActionOutcome, is_error: bool) -> ActionStatus {
    let raw_message = outcome.message.trim().to_string();
    let message = action_status_message(&raw_message, is_error);
    let detail = if message == raw_message || raw_message.is_empty() {
        outcome.detail
    } else {
        outcome.detail.or(Some(raw_message))
    };

    if is_error {
        ActionStatus::failed(message, detail)
    } else {
        ActionStatus::succeeded(message, detail)
    }
}

fn action_status_message(raw_message: &str, is_error: bool) -> String {
    if raw_message.is_empty() {
        return if is_error {
            "Network action failed. Open diagnostics for details.".into()
        } else {
            "Network action finished.".into()
        };
    }

    if raw_message.contains("Tailscale unavailable")
        || raw_message.contains("failed to connect to local Tailscale service")
    {
        return tailscale_unavailable_message(raw_message, is_error);
    }

    if raw_message.chars().count() <= ACTION_STATUS_SUMMARY_CHAR_LIMIT {
        return raw_message.to_string();
    }

    raw_message
        .split(['\n', ';'])
        .next()
        .map(str::trim)
        .filter(|summary| !summary.is_empty())
        .map(|summary| trim_action_summary(summary, ACTION_STATUS_SUMMARY_CHAR_LIMIT))
        .unwrap_or_else(|| {
            if is_error {
                "Network action failed. Open diagnostics for details.".into()
            } else {
                "Network action finished. Open diagnostics for details.".into()
            }
        })
}

fn tailscale_unavailable_message(raw_message: &str, is_error: bool) -> String {
    if is_error {
        return "Tailscale is unavailable. LAN discovery may still work.".into();
    }

    let sync_message = "Local sync finished. Tailscale is unavailable.";
    if raw_message.contains("Local daemon started") {
        return format!("Local daemon started. {sync_message}");
    }
    if raw_message.contains("Local daemon is already running") {
        return format!("Local daemon is already running. {sync_message}");
    }
    sync_message.into()
}

fn trim_action_summary(value: &str, max_chars: usize) -> String {
    let value = value.trim();
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let mut summary = value
        .chars()
        .take(max_chars.saturating_sub(1))
        .collect::<String>();
    summary.push('…');
    summary
}

impl Render for NetworkManagerApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.startup_backend_checked {
            self.startup_backend_checked = true;
            cx.defer_in(window, |app, _, cx| app.start_startup_backend(cx));
        }

        let tokens = self.tokens;
        let route = self.route;
        let action_status = self.action_status.clone();

        let content = match route {
            Route::Dashboard => {
                let vm = self.repository.dashboard();
                dashboard::screen(&vm, action_status.as_ref(), tokens, cx)
            }
            Route::Discovery => {
                let vm = self.repository.discovery();
                discovery::screen(
                    &vm,
                    self.selected_discovery_filter,
                    action_status.as_ref(),
                    tokens,
                    cx,
                )
            }
            Route::DeviceDetail => {
                let vm = self
                    .repository
                    .selected_device_detail(self.selected_device_id.as_deref());
                device_detail::screen(&vm, action_status.as_ref(), tokens, cx)
            }
            Route::QuickAccess => {
                let vm = self.repository.quick_access();
                quick_access::screen(&vm, action_status.as_ref(), tokens, cx)
            }
            Route::Settings => {
                let vm = self.repository.settings();
                settings::screen(&vm, action_status.as_ref(), tokens, cx)
            }
        };
        let focus_handle = self.ensure_focus_handle(window, cx);

        Self::root_shell(content, tokens, &focus_handle, cx)
    }
}

impl NetworkManagerApp {
    fn root_shell(
        content: impl IntoElement,
        tokens: LiquidGlassTokens,
        focus_handle: &FocusHandle,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        window_shell(content, tokens)
            .id("network-manager-root")
            .overflow_scroll()
            .key_context(KEY_CONTEXT)
            .track_focus(focus_handle)
            .on_action(
                cx.listener(|app, _: &ShowDashboard, _, cx| app.set_route(Route::Dashboard, cx)),
            )
            .on_action(
                cx.listener(|app, _: &ShowDiscovery, _, cx| app.set_route(Route::Discovery, cx)),
            )
            .on_action(cx.listener(|app, _: &ShowDeviceDetail, _, cx| {
                app.set_route(Route::DeviceDetail, cx)
            }))
            .on_action(
                cx.listener(|app, _: &ShowQuickAccess, _, cx| {
                    app.set_route(Route::QuickAccess, cx)
                }),
            )
            .on_action(cx.listener(|app, _: &NewWindow, _, cx| app.open_new_window(cx)))
            .on_action(
                cx.listener(|app, _: &ShowSettings, _, cx| app.set_route(Route::Settings, cx)),
            )
            .on_action(cx.listener(|app, _: &PreviousRoute, _, cx| app.previous_route(cx)))
            .on_action(cx.listener(|app, _: &NextRoute, _, cx| app.next_route(cx)))
            .on_action(cx.listener(|app, _: &RefreshQuick, _, cx| app.refresh_quick(cx)))
            .on_action(cx.listener(|app, _: &RefreshFull, _, cx| app.refresh_full(cx)))
            .on_action(
                cx.listener(|app, _: &ShowKeyboardShortcuts, _, cx| {
                    app.show_keyboard_shortcuts(cx)
                }),
            )
            .on_action(|_: &CloseWindow, window, _| window.remove_window())
            .on_action(|_: &MinimizeWindow, window, _| window.minimize_window())
            .on_action(|_: &ZoomWindow, window, _| window.zoom_window())
            .on_action(|_: &ToggleFullscreen, window, _| window.toggle_fullscreen())
            .on_action(|_: &BringAllToFront, window, _| window.activate_window())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{
        DaemonStatusVm, DashboardVm, DeviceDetailVm, DeviceIdentityVm, DiscoveryRowVm, DiscoveryVm,
        EndpointGroup, EndpointVm, QuickAccessVm, SettingsVm, SshTargetVm, TrackedDeviceRowVm,
    };
    use crate::hotkeys::install_app_hotkeys;
    use gpui::{Entity, TestAppContext, VisualTestContext};
    use network_manager_core::{AvailabilityState, EndpointKind, EndpointPreference};
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct FixtureRepository {
        reads: Arc<Mutex<Vec<&'static str>>>,
    }

    impl FixtureRepository {
        fn record(&self, screen: &'static str) {
            self.reads.lock().unwrap().push(screen);
        }
    }

    impl NetworkManagerRepository for FixtureRepository {
        fn dashboard(&self) -> DashboardVm {
            self.record("dashboard");
            DashboardVm {
                daemon: fixture_daemon(),
                tracked: vec![fixture_tracked_device()],
                online_count: 1,
                tailscale_count: 1,
            }
        }

        fn discovery(&self) -> DiscoveryVm {
            self.record("discovery");
            DiscoveryVm {
                rows: vec![DiscoveryRowVm {
                    id: "discovery-1".into(),
                    identity_id: Some("device-1".into()),
                    display_name: "Device One".into(),
                    hostname: "device.local".into(),
                    ip_address: "192.168.1.20".into(),
                    source: "mDNS".into(),
                    sources: vec!["LAN".into(), "Tailscale".into()],
                    category: "Computer".into(),
                    tracked_state: TrackedState::Untracked,
                    availability: AvailabilityState::Online,
                    ssh_capable: true,
                    last_seen: "now".into(),
                }],
            }
        }

        fn selected_device_detail(&self, selected_identity_id: Option<&str>) -> DeviceDetailVm {
            self.record("detail");
            fixture_detail(selected_identity_id.unwrap_or("device-1"))
        }

        fn quick_access(&self) -> QuickAccessVm {
            self.record("quick_access");
            QuickAccessVm {
                rows: vec![fixture_tracked_device()],
                last_scan: "now".into(),
            }
        }

        fn settings(&self) -> SettingsVm {
            self.record("settings");
            SettingsVm {
                daemon: fixture_daemon(),
            }
        }
    }

    fn fixture_daemon() -> DaemonStatusVm {
        DaemonStatusVm {
            state: AvailabilityState::Online,
            source: "fixture".into(),
            tailscale_service: AvailabilityState::Online,
            local_ip_address: "192.168.1.10".into(),
            last_scan: "now".into(),
            stale: false,
        }
    }

    fn fixture_tracked_device() -> TrackedDeviceRowVm {
        TrackedDeviceRowVm {
            id: "device-1".into(),
            label: "Device One".into(),
            alias: "device".into(),
            category: "Computer".into(),
            overall: AvailabilityState::Online,
            lan: AvailabilityState::Online,
            tailscale: AvailabilityState::Online,
            ssh: AvailabilityState::Online,
            preferred_target: "alice@device.local".into(),
            target_reason: "LAN endpoint is reachable".into(),
            last_seen: "now".into(),
        }
    }

    fn fixture_identity(id: &str) -> DeviceIdentityVm {
        DeviceIdentityVm {
            id: id.into(),
            label: "Device One".into(),
            alias: "device".into(),
            category: "Computer".into(),
            tracked_state: TrackedState::Tracked,
            availability: AvailabilityState::Online,
            ssh_username: Some("alice".into()),
            endpoint_preference: EndpointPreference::LanFirst,
        }
    }

    fn fixture_detail(id: &str) -> DeviceDetailVm {
        let preferred_target = (id != "no-target").then(|| SshTargetVm {
            destination: "alice@device.local".into(),
            command: "ssh -p 2222 alice@device.local".into(),
            reason: "LAN endpoint is reachable".into(),
        });
        DeviceDetailVm {
            identity: fixture_identity(id),
            device_list: vec![fixture_identity("device-1")],
            endpoints: vec![EndpointVm {
                id: "endpoint-1".into(),
                group: EndpointGroup::Lan,
                kind: EndpointKind::LanDns,
                address: "device.local".into(),
                hostname: Some("device.local".into()),
                port: Some(2222),
                reachability: AvailabilityState::Online,
                ssh_capability: AvailabilityState::Online,
                last_checked: "now".into(),
                preferred: true,
            }],
            preferred_target,
            evidence: vec!["mDNS observation".into()],
        }
    }

    #[derive(Clone, Default)]
    struct RecordingActions {
        calls: Arc<Mutex<Vec<String>>>,
        fail: bool,
    }

    impl NetworkManagerActions for RecordingActions {
        fn refresh(&self, mode: RefreshMode) -> anyhow::Result<ActionOutcome> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("refresh:{}", mode.as_str()));
            if self.fail {
                anyhow::bail!("refresh failed");
            }
            Ok(ActionOutcome::new("refresh ok"))
        }

        fn ensure_backend(&self) -> anyhow::Result<ActionOutcome> {
            self.calls.lock().unwrap().push("ensure_backend".into());
            if self.fail {
                anyhow::bail!("backend failed");
            }
            Ok(ActionOutcome::new("backend ok"))
        }

        fn set_tracked_state(
            &self,
            device_query: &str,
            state: TrackedState,
        ) -> anyhow::Result<ActionOutcome> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("track:{device_query}:{}", state.as_str()));
            if self.fail {
                anyhow::bail!("track failed");
            }
            Ok(ActionOutcome::new("track ok"))
        }

        fn daemon_lifecycle(&self, action: DaemonLifecycleAction) -> anyhow::Result<ActionOutcome> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("daemon:{}", action.label()));
            if self.fail {
                anyhow::bail!("daemon action failed");
            }
            Ok(ActionOutcome::new("daemon ok"))
        }

        fn open_diagnostics_folder(&self) -> anyhow::Result<ActionOutcome> {
            self.calls.lock().unwrap().push("open_diagnostics".into());
            if self.fail {
                anyhow::bail!("diagnostics failed");
            }
            Ok(ActionOutcome::new("diagnostics ok"))
        }
    }

    fn test_app() -> NetworkManagerApp {
        NetworkManagerApp::new_with_actions(
            FixtureRepository::default(),
            RecordingActions::default(),
        )
    }

    fn rendered_test_app(
        cx: &mut TestAppContext,
        repository: FixtureRepository,
        actions: RecordingActions,
    ) -> (Entity<NetworkManagerApp>, &mut VisualTestContext) {
        cx.update(install_app_hotkeys);
        cx.add_window_view(move |_, _| NetworkManagerApp::new_with_actions(repository, actions))
    }

    fn route_of(app: &Entity<NetworkManagerApp>, cx: &VisualTestContext) -> Route {
        cx.cx.read(|cx| app.read(cx).current_route())
    }

    fn discovery_filter_of(
        app: &Entity<NetworkManagerApp>,
        cx: &VisualTestContext,
    ) -> DiscoveryFilter {
        cx.cx.read(|cx| app.read(cx).current_discovery_filter())
    }

    fn selected_device_of(
        app: &Entity<NetworkManagerApp>,
        cx: &VisualTestContext,
    ) -> Option<String> {
        cx.cx
            .read(|cx| app.read(cx).selected_device_id().map(str::to_string))
    }

    fn action_status_of(
        app: &Entity<NetworkManagerApp>,
        cx: &VisualTestContext,
    ) -> Option<ActionStatus> {
        cx.cx.read(|cx| app.read(cx).action_status().cloned())
    }

    #[gpui::test]
    fn gpui_hotkeys_render_every_route_and_dispatch_refreshes(cx: &mut TestAppContext) {
        let reads = Arc::new(Mutex::new(Vec::new()));
        let calls = Arc::new(Mutex::new(Vec::new()));
        let repository = FixtureRepository {
            reads: reads.clone(),
        };
        let actions = RecordingActions {
            calls: calls.clone(),
            fail: false,
        };
        let (app, cx) = rendered_test_app(cx, repository, actions);

        assert_eq!(
            calls.lock().unwrap().as_slice(),
            ["ensure_backend", "refresh:quick"]
        );
        calls.lock().unwrap().clear();
        reads.lock().unwrap().clear();

        for (keystroke, route) in [
            ("cmd-1", Route::Dashboard),
            ("cmd-2", Route::Discovery),
            ("cmd-3", Route::DeviceDetail),
            ("cmd-4", Route::QuickAccess),
            ("cmd-5", Route::Settings),
        ] {
            cx.simulate_keystrokes(keystroke);
            assert_eq!(route_of(&app, cx), route, "{keystroke}");
        }
        let rendered = reads.lock().unwrap().clone();
        for screen in [
            "dashboard",
            "discovery",
            "detail",
            "quick_access",
            "settings",
        ] {
            assert!(rendered.contains(&screen), "{screen} did not render");
        }

        cx.simulate_keystrokes("cmd-]");
        assert_eq!(route_of(&app, cx), Route::Dashboard);
        cx.simulate_keystrokes("cmd-[");
        assert_eq!(route_of(&app, cx), Route::Settings);
        cx.simulate_keystrokes("cmd-k");
        assert_eq!(route_of(&app, cx), Route::QuickAccess);
        cx.simulate_keystrokes("escape");
        assert_eq!(route_of(&app, cx), Route::Dashboard);

        cx.dispatch_action(ShowDiscovery);
        assert_eq!(route_of(&app, cx), Route::Discovery);

        cx.simulate_keystrokes("cmd-r");
        cx.simulate_keystrokes("cmd-shift-r");
        assert_eq!(
            calls.lock().unwrap().as_slice(),
            ["refresh:quick", "refresh:full"]
        );
        assert_eq!(
            action_status_of(&app, cx).map(|status| status.message),
            Some("refresh ok".into())
        );

        cx.simulate_keystrokes("cmd-/");
        assert!(action_status_of(&app, cx)
            .is_some_and(|status| status.message.contains("⌘1 Dashboard")));
    }

    #[gpui::test]
    fn gpui_discovery_detail_and_quick_access_actions_use_real_app_state(cx: &mut TestAppContext) {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let actions = RecordingActions {
            calls: calls.clone(),
            fail: false,
        };
        let (app, cx) = rendered_test_app(cx, FixtureRepository::default(), actions);
        calls.lock().unwrap().clear();

        for filter in DiscoveryFilter::ALL {
            app.update(cx, |app, cx| app.select_discovery_filter(filter, cx));
            cx.run_until_parked();
            assert_eq!(route_of(&app, cx), Route::Discovery);
            assert_eq!(discovery_filter_of(&app, cx), filter);
        }

        app.update(cx, |app, cx| {
            app.set_discovery_identity_state(Some("device-1".into()), TrackedState::Tracked, cx)
        });
        cx.run_until_parked();
        app.update(cx, |app, cx| {
            app.set_discovery_identity_state(Some("device-1".into()), TrackedState::Untracked, cx)
        });
        cx.run_until_parked();
        assert_eq!(
            calls.lock().unwrap().as_slice(),
            ["track:device-1:tracked", "track:device-1:untracked"]
        );

        app.update(cx, |app, cx| {
            app.set_discovery_identity_state(None, TrackedState::Tracked, cx)
        });
        assert!(action_status_of(&app, cx).is_some_and(
            |status| status.is_error() && status.message.contains("no device identity")
        ));

        app.update(cx, |app, cx| {
            app.set_route(Route::QuickAccess, cx);
            app.select_device_detail("device-1".into(), cx);
        });
        cx.run_until_parked();
        assert_eq!(route_of(&app, cx), Route::DeviceDetail);
        assert_eq!(selected_device_of(&app, cx).as_deref(), Some("device-1"));

        app.update(cx, |app, cx| app.copy_selected_ssh_command(cx));
        assert_eq!(
            cx.read_from_clipboard().and_then(|item| item.text()),
            Some("ssh -p 2222 alice@device.local".into())
        );
        assert_eq!(
            action_status_of(&app, cx).map(|status| status.message),
            Some("Copied ssh -p 2222 alice@device.local".into())
        );

        app.update(cx, |app, cx| app.copy_selected_target(cx));
        assert_eq!(
            cx.read_from_clipboard().and_then(|item| item.text()),
            Some("alice@device.local".into())
        );

        app.update(cx, |app, cx| {
            app.select_device_detail("no-target".into(), cx)
        });
        app.update(cx, |app, cx| app.copy_selected_ssh_command(cx));
        assert!(action_status_of(&app, cx)
            .is_some_and(|status| status.is_error() && status.message.contains("No SSH target")));
    }

    #[gpui::test]
    fn gpui_settings_lifecycle_and_safe_window_actions_are_wired(cx: &mut TestAppContext) {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let actions = RecordingActions {
            calls: calls.clone(),
            fail: false,
        };
        let (app, cx) = rendered_test_app(cx, FixtureRepository::default(), actions);
        calls.lock().unwrap().clear();

        cx.dispatch_action(ShowSettings);
        app.update(cx, |app, cx| app.install_and_start_daemon(cx));
        cx.run_until_parked();
        app.update(cx, |app, cx| app.start_daemon(cx));
        cx.run_until_parked();
        app.update(cx, |app, cx| app.restart_daemon(cx));
        cx.run_until_parked();
        app.update(cx, |app, cx| app.stop_daemon(cx));
        cx.run_until_parked();
        app.update(cx, |app, cx| app.open_diagnostics_folder(cx));
        cx.run_until_parked();

        assert_eq!(route_of(&app, cx), Route::Settings);
        assert_eq!(
            calls.lock().unwrap().as_slice(),
            [
                "daemon:install daemon",
                "daemon:start daemon",
                "daemon:restart daemon",
                "daemon:stop daemon",
                "open_diagnostics"
            ]
        );
        assert_eq!(
            action_status_of(&app, cx).map(|status| status.message),
            Some("diagnostics ok".into())
        );

        assert!(!cx.update(|window, _| window.is_fullscreen()));
        cx.dispatch_action(ToggleFullscreen);
        assert!(cx.update(|window, _| window.is_fullscreen()));
        cx.dispatch_action(ToggleFullscreen);
        assert!(!cx.update(|window, _| window.is_fullscreen()));

        cx.deactivate_window();
        assert!(!cx.update(|window, _| window.is_window_active()));
        cx.dispatch_action(BringAllToFront);
        assert!(cx.update(|window, _| window.is_window_active()));

        cx.dispatch_action(CloseWindow);
        assert!(cx.windows().is_empty());
    }

    #[test]
    fn route_selection_keeps_all_screens_reachable() {
        let mut app = test_app();
        for route in Route::ALL {
            app.select_route_for_test(route);
            assert_eq!(app.current_route(), route);
        }
    }

    #[test]
    fn action_gateway_records_success_and_errors() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let actions = RecordingActions {
            calls: calls.clone(),
            fail: false,
        };
        let mut app = NetworkManagerApp::new_with_actions(FixtureRepository::default(), actions);

        app.refresh_for_test(RefreshMode::Full);
        app.set_discovery_identity_state_for_test(Some("identity-1".into()), TrackedState::Tracked);

        assert_eq!(
            calls.lock().unwrap().as_slice(),
            ["refresh:full", "track:identity-1:tracked"]
        );
        assert_eq!(
            app.action_status().map(|status| status.message.as_str()),
            Some("track ok")
        );
        assert_eq!(app.action_status().map(ActionStatus::is_error), Some(false));

        app.set_discovery_identity_state_for_test(None, TrackedState::Tracked);
        assert_eq!(app.action_status().map(ActionStatus::is_error), Some(true));
    }

    #[test]
    fn action_gateway_routes_daemon_lifecycle_actions() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let actions = RecordingActions {
            calls: calls.clone(),
            fail: false,
        };
        let mut app = NetworkManagerApp::new_with_actions(FixtureRepository::default(), actions);

        app.daemon_lifecycle_for_test(DaemonLifecycleAction::InstallAndStart);
        app.daemon_lifecycle_for_test(DaemonLifecycleAction::Start);
        app.daemon_lifecycle_for_test(DaemonLifecycleAction::Restart);
        app.daemon_lifecycle_for_test(DaemonLifecycleAction::Stop);

        assert_eq!(
            calls.lock().unwrap().as_slice(),
            [
                "daemon:install daemon",
                "daemon:start daemon",
                "daemon:restart daemon",
                "daemon:stop daemon"
            ]
        );
        assert_eq!(
            app.action_status().map(|status| status.message.as_str()),
            Some("daemon ok")
        );
    }

    #[test]
    fn action_gateway_routes_diagnostics_folder_action() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let actions = RecordingActions {
            calls: calls.clone(),
            fail: false,
        };
        let mut app = NetworkManagerApp::new_with_actions(FixtureRepository::default(), actions);

        app.open_diagnostics_folder_for_test();

        assert_eq!(calls.lock().unwrap().as_slice(), ["open_diagnostics"]);
        assert_eq!(
            app.action_status().map(|status| status.message.as_str()),
            Some("diagnostics ok")
        );
    }

    #[test]
    fn action_gateway_surfaces_daemon_errors() {
        let actions = RecordingActions {
            calls: Arc::new(Mutex::new(Vec::new())),
            fail: true,
        };
        let mut app = NetworkManagerApp::new_with_actions(FixtureRepository::default(), actions);

        app.refresh_for_test(RefreshMode::Quick);

        let status = app.action_status().unwrap();
        assert!(status.is_error());
        assert!(status.message.contains("refresh failed"));
    }

    #[test]
    fn action_gateway_preserves_action_outcome_detail() {
        let raw = "started bundled daemon; quick refresh: Tailscale unavailable: tailscale status failed: failed to connect to local Tailscale service; is Tailscale running?; recorded 7 mDNS services".to_string();
        let outcome = ActionOutcome::new(raw.clone());

        let status = action_status_from_outcome(outcome, false);

        assert_eq!(
            status.message,
            "Local sync finished. Tailscale is unavailable."
        );
        assert_eq!(status.detail.as_deref(), Some(raw.as_str()));
        assert!(!status.is_error());
    }

    #[test]
    fn action_gateway_keeps_backend_start_in_tailscale_summary() {
        let raw = "quick refresh: Tailscale unavailable: tailscale status failed";
        let outcome = ActionOutcome::combine(
            ActionOutcome::new("Local daemon started."),
            ActionOutcome::new(raw),
        );

        let status = action_status_from_outcome(outcome, false);

        assert_eq!(
            status.message,
            "Local daemon started. Local sync finished. Tailscale is unavailable."
        );
        assert_eq!(
            status.detail.as_deref(),
            Some("Local daemon started. quick refresh: Tailscale unavailable: tailscale status failed")
        );
        assert!(!status.is_error());
    }

    #[test]
    fn action_gateway_summarizes_tailscale_errors_as_recoverable() {
        let raw =
            "tailscale status failed: failed to connect to local Tailscale service; is Tailscale running?";

        let status = action_status_from_outcome(ActionOutcome::new(raw), true);

        assert_eq!(
            status.message,
            "Tailscale is unavailable. LAN discovery may still work."
        );
        assert_eq!(status.detail.as_deref(), Some(raw));
        assert!(status.is_error());
    }

    #[test]
    fn action_gateway_keeps_short_action_outcomes_without_detail() {
        let status = action_status_from_outcome(ActionOutcome::new("refresh ok"), false);

        assert_eq!(status.message, "refresh ok");
        assert_eq!(status.detail, None);
        assert!(!status.is_error());
    }

    #[test]
    fn stale_async_action_results_do_not_overwrite_current_status() {
        let mut app = test_app();
        let stale_action = app.next_action_sequence();
        let current_action = app.next_action_sequence();
        app.action_status = Some(ActionStatus::pending("current action running"));

        app.record_action_result_if_current(
            stale_action,
            Ok(ActionOutcome::new("stale action complete")),
        );
        assert_eq!(
            app.action_status().map(|status| status.message.as_str()),
            Some("current action running")
        );
        assert_eq!(
            app.action_status().map(ActionStatus::is_pending),
            Some(true)
        );

        app.record_action_result_if_current(
            current_action,
            Ok(ActionOutcome::new("current action complete")),
        );
        assert_eq!(
            app.action_status().map(|status| status.message.as_str()),
            Some("current action complete")
        );
        assert_eq!(
            app.action_status().map(ActionStatus::is_pending),
            Some(false)
        );
    }
}
