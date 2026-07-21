#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoveryFilter {
    AllSources,
    Lan,
    Tailscale,
    SshCapable,
    Untracked,
}

impl DiscoveryFilter {
    pub const ALL: [DiscoveryFilter; 5] = [
        DiscoveryFilter::AllSources,
        DiscoveryFilter::Lan,
        DiscoveryFilter::Tailscale,
        DiscoveryFilter::SshCapable,
        DiscoveryFilter::Untracked,
    ];

    pub fn label(self) -> &'static str {
        match self {
            DiscoveryFilter::AllSources => "All sources",
            DiscoveryFilter::Lan => "LAN",
            DiscoveryFilter::Tailscale => "Tailscale",
            DiscoveryFilter::SshCapable => "SSH capable",
            DiscoveryFilter::Untracked => "Untracked",
        }
    }
}

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
}
