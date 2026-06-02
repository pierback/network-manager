#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Dashboard,
    Discovery,
    DeviceDetail,
    QuickAccess,
    Settings,
}

impl Route {
    pub const ALL: [Route; 5] = [
        Route::Dashboard,
        Route::Discovery,
        Route::DeviceDetail,
        Route::QuickAccess,
        Route::Settings,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Route::Dashboard => "Dashboard",
            Route::Discovery => "Discovery",
            Route::DeviceDetail => "Device Detail",
            Route::QuickAccess => "Quick Access",
            Route::Settings => "Settings",
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Route::Dashboard => "◇",
            Route::Discovery => "⌕",
            Route::DeviceDetail => "◧",
            Route::QuickAccess => "⌘",
            Route::Settings => "⚙",
        }
    }
}
