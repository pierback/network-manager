use gpui::{prelude::*, Context, Render, Window};

use crate::data::{MockRepository, NetworkManagerRepository};
use crate::layout::app_shell::{app_body, content_frame, sidebar, window_shell};
use crate::routes::Route;
use crate::screens::{dashboard, device_detail, discovery, quick_access, settings};
use crate::theme::LiquidGlassTokens;

pub struct NetworkManagerApp {
    route: Route,
    repository: Box<dyn NetworkManagerRepository>,
    tokens: LiquidGlassTokens,
}

impl NetworkManagerApp {
    pub fn new(repository: impl NetworkManagerRepository + 'static) -> Self {
        Self {
            route: Route::Dashboard,
            repository: Box::new(repository),
            tokens: LiquidGlassTokens::v4(),
        }
    }

    pub fn mock() -> Self {
        Self::new(MockRepository::new())
    }

    pub fn current_route(&self) -> Route {
        self.route
    }

    pub(crate) fn set_route(&mut self, route: Route, cx: &mut Context<Self>) {
        self.route = route;
        cx.notify();
    }

    pub fn select_route_for_test(&mut self, route: Route) {
        self.route = route;
    }
}

impl Render for NetworkManagerApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tokens = self.tokens;
        let route = self.route;
        let dashboard_vm = self.repository.dashboard();
        let discovery_vm = self.repository.discovery();
        let detail_vm = self.repository.selected_device_detail();
        let quick_vm = self.repository.quick_access();
        let settings_vm = self.repository.settings();

        let content = match route {
            Route::Dashboard => dashboard::screen(&dashboard_vm, tokens),
            Route::Discovery => discovery::screen(&discovery_vm, tokens),
            Route::DeviceDetail => device_detail::screen(&detail_vm, tokens),
            Route::QuickAccess => quick_access::screen(&quick_vm, tokens),
            Route::Settings => settings::screen(&settings_vm, tokens),
        };

        let nav = sidebar(route, tokens, cx);

        window_shell(app_body(nav, content_frame(content)), tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_selection_keeps_all_screens_reachable() {
        let mut app = NetworkManagerApp::mock();
        for route in Route::ALL {
            app.select_route_for_test(route);
            assert_eq!(app.current_route(), route);
        }
    }
}
