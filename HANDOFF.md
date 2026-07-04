# Network Manager Handoff

Date: 2026-07-04

## Repository

- Path: `/Users/f.pieringer/projects/network-manager`
- GitHub: https://github.com/pierback/network-manager
- Branch: `main`

## Purpose

Network Manager is a macOS-only Rust app, CLI, and user daemon for discovering
LAN, mDNS, and Tailscale devices, tracking important machines, and resolving
the best SSH target for humans or agents.

## Setup and Run

```bash
cargo build --workspace
cargo run -p network-manager-daemon
cargo run -p network-manager-cli -- daemon status
cargo run -p network-manager-cli -- refresh --quick
cargo run -p network-manager-cli -- list
cargo run -p network-manager-ui
```

## Verification

```bash
cargo fmt
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --workspace
```

## Current State

- Existing origin is `https://github.com/pierback/network-manager.git`.
- The workspace was clean before this handoff file was added.
- This checkpoint commit exists to make the local project state recoverable from
  GitHub on another Mac.

## Risks and Open Loops

- Tailscale discovery depends on the local Tailscale service and CLI.
- LAN ARP and mDNS discovery degrade independently when helper tools or network
  services are unavailable.
- Public distribution still requires Developer ID signing and notarization
  after local app packaging.

## Next Steps on Another Mac

1. Clone `https://github.com/pierback/network-manager`.
2. Install a current Rust toolchain and macOS build prerequisites.
3. Run the verification commands above.
4. Start the daemon with a temporary database/socket for smoke testing before
   installing the LaunchAgent.
