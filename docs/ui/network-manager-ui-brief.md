# Network Manager UI Design Brief

Create a native-feeling macOS desktop app design for Network Manager, a local app that helps the user identify devices observable from the Mac, select the devices they care about, and quickly resolve/SSH into them.

## Product shape

- Rust + GPUI desktop app, not Electron.
- Includes both a dashboard window and quick-access/menu-bar experience.
- CLI is first-class, but this design is for the graphical app.
- All data stays local.

## Core concepts to visualize

- **Tracked Devices**: selected devices shown on the main dashboard.
- **Discovered Devices**: all observable devices from LAN/Tailscale discovery, shown in Discovery.
- **Device Identity**: one stable device across changing endpoints.
- **Device Label**: user-facing display name, e.g. “Office MacBook”.
- **Device Alias**: CLI-friendly unique name, e.g. `office-macbook`.
- **Network Endpoints**: LAN IP, LAN DNS/mDNS name, Tailscale DNS, Tailscale IP.
- **Availability State**: online, offline, unknown, stale/degraded.
- **Network Proximity**: whether LAN/local route is usable now; prefer LAN SSH target when local, Tailscale when remote.
- **SSH Capability**: separate from reachability.

## Required screens / artboards

1. **Dashboard**
   - Shows only Tracked Devices.
   - Cards or table rows with label, alias, category, overall availability, LAN status, Tailscale status, SSH status, preferred SSH target, last seen, quick actions.
   - Quick actions: SSH, copy target, show details, refresh.
   - Color/status mapping: green online, red offline, gray unknown, yellow stale/degraded.
   - Empty state that directs user to Discovery.

2. **Discovery**
   - Shows all discovered devices, tracked and untracked.
   - Filters: source, status, category, tracked/untracked.
   - Search field.
   - Rows show evidence/source badges: LAN, mDNS, DNS, ARP, Tailscale.
   - Action to track/untrack.
   - “Possible matches”/merge suggestions with confidence and why.

3. **Device Detail / Inspector**
   - Device label and alias editable.
   - Endpoint list grouped by LAN and Tailscale.
   - Statuses shown separately: local Tailscale service, Tailscale presence, endpoint reachability, SSH capability.
   - Shows selected/preferred SSH target with reason: e.g. “Using LAN because local reachability was confirmed”.
   - Merge/split controls with evidence explanation.

4. **Quick Access / Menu Bar Popover**
   - Compact list of tracked devices.
   - Status dots.
   - Actions: SSH, copy endpoint, refresh, open dashboard.
   - Designed for fast glanceability.

5. **Settings**
   - Discovery cadence and battery behavior.
   - Tailscale availability status.
   - CLI path/help, `network-manager paths`, optional SSH config export.
   - Privacy/logging controls.

## Visual direction

- Native macOS productivity app, compact but polished.
- No marketing/landing-page style.
- Clear hierarchy, muted surfaces, subtle borders, sidebar navigation.
- Dark mode preferred, but include light-mode-compatible tokens if possible.
- Use realistic sample devices:
  - Office MacBook — alias `office-macbook`, category Mac, LAN + Tailscale online, SSH online.
  - Living Room Apple TV — category Media, LAN online, SSH unavailable.
  - Printer — LAN online, Tailscale absent, SSH unavailable.
  - NAS — LAN stale, Tailscale online, SSH online.
  - iPhone — mDNS seen recently, unknown SSH.

## Deliverable

Create a new `.pen` design file with multiple labeled artboards and export a preview image.
