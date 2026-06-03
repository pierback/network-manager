pub const DASHBOARD_COLUMNS: [(&str, f32); 9] = [
    ("", 8.0),
    ("DEVICE", 180.0),
    ("CATEGORY", 80.0),
    ("LAN", 70.0),
    ("TAILSCALE", 80.0),
    ("SSH", 70.0),
    ("SSH TARGET", 140.0),
    ("LAST SEEN", 80.0),
    ("", 104.0),
];

pub const DISCOVERY_COLUMNS: [(&str, f32); 6] = [
    ("DEVICE", 200.0),
    ("HOSTNAME", 160.0),
    ("IP ADDRESS", 120.0),
    ("SOURCES", 180.0),
    ("STATUS", 90.0),
    ("", 120.0),
];
