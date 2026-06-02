use super::{DashboardVm, DeviceDetailVm, DiscoveryVm, QuickAccessVm, SettingsVm};

pub trait NetworkManagerRepository {
    fn dashboard(&self) -> DashboardVm;
    fn discovery(&self) -> DiscoveryVm;
    fn selected_device_detail(&self) -> DeviceDetailVm;
    fn quick_access(&self) -> QuickAccessVm;
    fn settings(&self) -> SettingsVm;
}
