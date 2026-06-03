# Network Manager

Network Manager is a macOS-only Rust app/CLI/daemon for discovering devices on the local network and Tailscale, tracking the devices that matter, and resolving the best SSH target for agents or humans.

## What is included

- `network-manager-daemon`: user daemon with bounded LAN/mDNS/Tailscale discovery, SSH endpoint probing, and typed gRPC over a Unix domain socket.
- `network-manager`: CLI for discovery, tracking, labels/aliases/categories/tags, merge/split corrections, refreshes, SSH resolution, LaunchAgent management, export/import, and SSH config generation.
- `network-manager-ui`: GPUI desktop shell backed by the same SQLite store, with daemon-backed refresh and discovery tracking actions.

## Build and validate

```bash
cargo fmt
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
```

## Run locally

Start the daemon in one terminal:

```bash
cargo run -p network-manager-daemon
```

Use the CLI in another terminal:

```bash
cargo run -p network-manager-cli -- daemon status
cargo run -p network-manager-cli -- refresh --quick
cargo run -p network-manager-cli -- list
cargo run -p network-manager-cli -- discover list
```

Run the UI:

```bash
cargo run -p network-manager-ui
```

## Default local paths

- SQLite database: `~/Library/Application Support/Network Manager/network-manager.sqlite`
- Unix socket: `${TMPDIR}/network-manager-${USER}.sock`
- LaunchAgent plist: `~/Library/LaunchAgents/com.network-manager.daemon.plist`
- Daemon logs: `~/Library/Logs/Network Manager/daemon.log` and `daemon.err.log`

You can override the database and socket with `--db`, `--socket`, `NETWORK_MANAGER_DB`, and `NETWORK_MANAGER_SOCKET`.

## LaunchAgent

Preview the per-user LaunchAgent plist:

```bash
cargo run -p network-manager-cli -- daemon plist \
  --daemon-path "$PWD/target/debug/network-manager-daemon"
```

Install and start it:

```bash
cargo build -p network-manager-daemon -p network-manager-cli
cargo run -p network-manager-cli -- daemon install \
  --daemon-path "$PWD/target/debug/network-manager-daemon" \
  --refresh-interval-seconds 60 \
  --force \
  --load
```

Manage it later:

```bash
network-manager daemon status
network-manager daemon stop
network-manager daemon start
network-manager daemon uninstall
```

The generated plist sets a LaunchAgent-friendly `PATH` including `/opt/homebrew/bin` and `/usr/local/bin` so helper commands such as `tailscale` can be found.

## Useful CLI flows

Track a discovered identity:

```bash
network-manager discover list
network-manager track <identity-or-discovered-name> --alias office-mac
```

Resolve or open SSH:

```bash
network-manager resolve office-mac
network-manager ssh office-mac -- uname -a
```

Generate reviewed SSH config entries without modifying `~/.ssh/config` automatically:

```bash
network-manager ssh-config > ~/.ssh/network-manager.generated
```

Then add this manually to `~/.ssh/config` if desired:

```sshconfig
Include ~/.ssh/network-manager.generated
```

Export/import portable user settings:

```bash
network-manager export network-manager-settings.json
network-manager import network-manager-settings.json --dry-run
network-manager import network-manager-settings.json
```

## Package the macOS app

Create an unsigned/ad-hoc signed `.app` bundle with the UI binary plus bundled CLI/daemon helpers:

```bash
scripts/package-app.sh
open "dist/Network Manager.app"
```

The Settings screen can install/start/stop the bundled per-user LaunchAgent when the app bundle contains `network-manager` and `network-manager-daemon` in `Contents/Resources`.

For public distribution, sign with a Developer ID certificate and notarize the resulting archive after this local packaging step.

## Smoke test with temporary state

```bash
tmpdir=$(mktemp -d)
cargo build -p network-manager-cli -p network-manager-daemon
./target/debug/network-manager-daemon --db "$tmpdir/nm.sqlite" --socket "$tmpdir/nm.sock" --disable-auto-refresh &
daemon_pid=$!
./target/debug/network-manager --db "$tmpdir/nm.sqlite" --socket "$tmpdir/nm.sock" --require-daemon daemon status
./target/debug/network-manager --db "$tmpdir/nm.sqlite" --socket "$tmpdir/nm.sock" refresh --quick
./target/debug/network-manager --db "$tmpdir/nm.sqlite" --socket "$tmpdir/nm.sock" list
kill "$daemon_pid"
```

Tailscale discovery requires the local Tailscale service and CLI to be available. LAN ARP and mDNS discovery degrade independently if Tailscale is unavailable.
