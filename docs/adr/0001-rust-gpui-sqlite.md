# Use Rust, GPUI, and SQLite for the local app

We will build Network Manager as a Rust application with a shared Rust core, a fast Rust CLI, a GPUI desktop interface, and a local SQLite store. This deliberately rejects Electron and avoids a SwiftUI-first architecture because the CLI must be first-class, minimal, and agent-friendly, while the desktop app can still provide a polished macOS dashboard and quick-access experience through GPUI.

## Considered Options

- SwiftUI app with Swift CLI: strongest native macOS fit, but risks placing core behavior too close to the UI and making the CLI less central.
- Rust core/CLI with SwiftUI shell: good split between CLI performance and native UI, but adds bridge complexity.
- Rust core/CLI with GPUI desktop app: keeps the product in one language, makes the CLI first-class, and shares discovery, identity matching, SSH resolution, and persistence logic directly.

## Consequences

SQLite is the shared local source of truth for discovered devices, tracked selection, labels, endpoints, identity corrections, and last-seen/status data. The UI must not own discovery, identity matching, persistence, or SSH target resolution logic.
