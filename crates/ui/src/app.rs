use std::sync::Arc;

use gpui::{prelude::*, Context, Render, Window};
use network_manager_core::TrackedState;

use crate::data::{
    ActionOutcome, DaemonActions, DaemonLifecycleAction, MockRepository, NetworkManagerActions,
    NetworkManagerRepository, NoopActions, RefreshMode, SqliteRepository,
};
use crate::layout::app_shell::{app_body, content_frame, sidebar, window_shell};
use crate::routes::Route;
use crate::screens::{dashboard, device_detail, discovery, quick_access, settings};
use crate::theme::LiquidGlassTokens;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionStatus {
    pub message: String,
    pub is_error: bool,
    pub is_pending: bool,
}

pub struct NetworkManagerApp {
    route: Route,
    repository: Box<dyn NetworkManagerRepository>,
    actions: Arc<dyn NetworkManagerActions>,
    selected_device_id: Option<String>,
    startup_backend_checked: bool,
    action_status: Option<ActionStatus>,
    action_sequence: u64,
    tokens: LiquidGlassTokens,
}

impl NetworkManagerApp {
    pub fn new(repository: impl NetworkManagerRepository + 'static) -> Self {
        Self::new_with_actions(repository, NoopActions)
    }

    pub fn new_with_actions(
        repository: impl NetworkManagerRepository + 'static,
        actions: impl NetworkManagerActions + 'static,
    ) -> Self {
        Self {
            route: Route::Dashboard,
            repository: Box::new(repository),
            actions: Arc::new(actions),
            selected_device_id: None,
            startup_backend_checked: false,
            action_status: None,
            action_sequence: 0,
            tokens: LiquidGlassTokens::v4(),
        }
    }

    pub fn live() -> Self {
        Self::new_with_actions(SqliteRepository::default(), DaemonActions::default())
    }

    pub fn mock() -> Self {
        Self::new(MockRepository::new())
    }

    pub fn current_route(&self) -> Route {
        self.route
    }

    pub fn action_status(&self) -> Option<&ActionStatus> {
        self.action_status.as_ref()
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

    pub fn select_route_for_test(&mut self, route: Route) {
        self.route = route;
    }

    pub fn refresh_for_test(&mut self, mode: RefreshMode) {
        self.refresh_inner(mode);
    }

    pub fn track_discovery_identity_for_test(&mut self, identity_id: Option<String>) {
        self.set_discovery_identity_state_inner(identity_id, TrackedState::Tracked);
    }

    pub fn set_discovery_identity_state_for_test(
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
            self.action_status = Some(ActionStatus {
                message: "Cannot update this discovery yet because it has no device identity."
                    .into(),
                is_error: true,
                is_pending: false,
            });
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

    #[allow(dead_code)]
    pub(crate) fn refresh_device(&mut self, identity_id: String, cx: &mut Context<Self>) {
        if identity_id.is_empty() || identity_id == "empty" {
            self.start_refresh(RefreshMode::Quick, cx);
            return;
        }
        self.start_action(format!("Refreshing {identity_id}…"), cx, move |actions| {
            actions.refresh_device(RefreshMode::Quick, &identity_id)
        });
    }

    #[allow(dead_code)]
    pub(crate) fn split_discovered_device(
        &mut self,
        discovered_device_id: String,
        cx: &mut Context<Self>,
    ) {
        self.start_action(
            format!("Splitting {discovered_device_id}…"),
            cx,
            move |actions| actions.split_discovered_device(&discovered_device_id),
        );
    }

    pub(crate) fn install_and_start_daemon(&mut self, cx: &mut Context<Self>) {
        self.start_daemon_lifecycle(DaemonLifecycleAction::InstallAndStart, cx);
    }

    pub(crate) fn start_daemon(&mut self, cx: &mut Context<Self>) {
        self.start_daemon_lifecycle(DaemonLifecycleAction::Start, cx);
    }

    pub(crate) fn stop_daemon(&mut self, cx: &mut Context<Self>) {
        self.start_daemon_lifecycle(DaemonLifecycleAction::Stop, cx);
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
                Ok(ActionOutcome {
                    message: format!("{}; {}", backend.message, refresh.message),
                })
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
        self.action_status = Some(ActionStatus {
            message: pending_message,
            is_error: false,
            is_pending: true,
        });
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

    fn refresh_inner(&mut self, mode: RefreshMode) {
        let result = self.actions.refresh(mode);
        self.record_action_result(result);
    }

    fn set_discovery_identity_state_inner(
        &mut self,
        identity_id: Option<String>,
        state: TrackedState,
    ) {
        let Some(identity_id) = identity_id else {
            self.invalidate_pending_action();
            self.action_status = Some(ActionStatus {
                message: "Cannot update this discovery yet because it has no device identity."
                    .into(),
                is_error: true,
                is_pending: false,
            });
            return;
        };
        let result = self.actions.set_tracked_state(&identity_id, state);
        self.record_action_result(result);
    }

    fn record_busy_action(&mut self, cx: &mut Context<Self>) -> bool {
        if self
            .action_status
            .as_ref()
            .is_some_and(|status| status.is_pending)
        {
            self.action_status = Some(ActionStatus {
                message: "Another network action is already running; wait for it to finish.".into(),
                is_error: false,
                is_pending: true,
            });
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
            Ok(outcome) => ActionStatus {
                message: outcome.message,
                is_error: false,
                is_pending: false,
            },
            Err(error) => ActionStatus {
                message: format!("{error:#}"),
                is_error: true,
                is_pending: false,
            },
        });
    }
}

impl Render for NetworkManagerApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.startup_backend_checked {
            self.startup_backend_checked = true;
            cx.defer_in(window, |app, _, cx| app.start_startup_backend(cx));
        }

        let tokens = self.tokens;
        let route = self.route;
        let dashboard_vm = self.repository.dashboard();
        let discovery_vm = self.repository.discovery();
        let detail_vm = self
            .repository
            .selected_device_detail(self.selected_device_id.as_deref());
        let quick_vm = self.repository.quick_access();
        let settings_vm = self.repository.settings();
        let action_status = self.action_status.clone();

        let content = match route {
            Route::Dashboard => {
                dashboard::screen(&dashboard_vm, action_status.as_ref(), tokens, cx)
            }
            Route::Discovery => {
                discovery::screen(&discovery_vm, action_status.as_ref(), tokens, cx)
            }
            Route::DeviceDetail => {
                device_detail::screen(&detail_vm, action_status.as_ref(), tokens, cx)
            }
            Route::QuickAccess => {
                quick_access::screen(&quick_vm, action_status.as_ref(), tokens, cx)
            }
            Route::Settings => settings::screen(&settings_vm, action_status.as_ref(), tokens, cx),
        };

        let nav = sidebar(route, tokens, cx);

        window_shell(app_body(nav, content_frame(content)), tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Clone)]
    struct RecordingActions {
        calls: Arc<Mutex<Vec<String>>>,
        fail: bool,
    }

    impl NetworkManagerActions for RecordingActions {
        fn refresh(&self, mode: RefreshMode) -> anyhow::Result<ActionOutcome> {
            self.refresh_device(mode, "")
        }

        fn refresh_device(
            &self,
            mode: RefreshMode,
            device_query: &str,
        ) -> anyhow::Result<ActionOutcome> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("refresh:{}:{device_query}", mode.as_str()));
            if self.fail {
                anyhow::bail!("refresh failed");
            }
            Ok(ActionOutcome {
                message: "refresh ok".into(),
            })
        }

        fn ensure_backend(&self) -> anyhow::Result<ActionOutcome> {
            self.calls.lock().unwrap().push("ensure_backend".into());
            if self.fail {
                anyhow::bail!("backend failed");
            }
            Ok(ActionOutcome {
                message: "backend ok".into(),
            })
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
            Ok(ActionOutcome {
                message: "track ok".into(),
            })
        }

        fn merge_identities(
            &self,
            source_query: &str,
            target_query: &str,
        ) -> anyhow::Result<ActionOutcome> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("merge:{source_query}:{target_query}"));
            if self.fail {
                anyhow::bail!("merge failed");
            }
            Ok(ActionOutcome {
                message: "merge ok".into(),
            })
        }

        fn split_discovered_device(
            &self,
            discovered_device_id: &str,
        ) -> anyhow::Result<ActionOutcome> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("split:{discovered_device_id}"));
            if self.fail {
                anyhow::bail!("split failed");
            }
            Ok(ActionOutcome {
                message: "split ok".into(),
            })
        }

        fn daemon_lifecycle(&self, action: DaemonLifecycleAction) -> anyhow::Result<ActionOutcome> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("daemon:{}", action.label()));
            if self.fail {
                anyhow::bail!("daemon action failed");
            }
            Ok(ActionOutcome {
                message: "daemon ok".into(),
            })
        }
    }

    #[test]
    fn route_selection_keeps_all_screens_reachable() {
        let mut app = NetworkManagerApp::mock();
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
        let mut app = NetworkManagerApp::new_with_actions(MockRepository::new(), actions);

        app.refresh_for_test(RefreshMode::Full);
        app.track_discovery_identity_for_test(Some("identity-1".into()));

        assert_eq!(
            calls.lock().unwrap().as_slice(),
            ["refresh:full:", "track:identity-1:tracked"]
        );
        assert_eq!(
            app.action_status().map(|status| status.message.as_str()),
            Some("track ok")
        );
        assert_eq!(
            app.action_status().map(|status| status.is_error),
            Some(false)
        );

        app.track_discovery_identity_for_test(None);
        assert_eq!(
            app.action_status().map(|status| status.is_error),
            Some(true)
        );
    }

    #[test]
    fn action_gateway_surfaces_daemon_errors() {
        let actions = RecordingActions {
            calls: Arc::new(Mutex::new(Vec::new())),
            fail: true,
        };
        let mut app = NetworkManagerApp::new_with_actions(MockRepository::new(), actions);

        app.refresh_for_test(RefreshMode::Quick);

        let status = app.action_status().unwrap();
        assert!(status.is_error);
        assert!(status.message.contains("refresh failed"));
    }

    #[test]
    fn stale_async_action_results_do_not_overwrite_current_status() {
        let mut app = NetworkManagerApp::mock();
        let stale_action = app.next_action_sequence();
        let current_action = app.next_action_sequence();
        app.action_status = Some(ActionStatus {
            message: "current action running".into(),
            is_error: false,
            is_pending: true,
        });

        app.record_action_result_if_current(
            stale_action,
            Ok(ActionOutcome {
                message: "stale action complete".into(),
            }),
        );
        assert_eq!(
            app.action_status().map(|status| status.message.as_str()),
            Some("current action running")
        );
        assert_eq!(
            app.action_status().map(|status| status.is_pending),
            Some(true)
        );

        app.record_action_result_if_current(
            current_action,
            Ok(ActionOutcome {
                message: "current action complete".into(),
            }),
        );
        assert_eq!(
            app.action_status().map(|status| status.message.as_str()),
            Some("current action complete")
        );
        assert_eq!(
            app.action_status().map(|status| status.is_pending),
            Some(false)
        );
    }
}
