# Use a user daemon with local gRPC over Unix domain sockets

Network Manager will run a user-level background daemon that owns discovery and status refreshes, writes discovery/status data to SQLite, and exposes a local IPC API to the GUI and CLI. The API will use Tonic gRPC over a Unix domain socket so we get typed Rust RPC and streaming without opening a TCP port or hand-rolling message parsing.

## Considered Options

- GUI/CLI scan directly: simpler at first, but duplicates discovery logic and makes status slow or stale.
- Localhost TCP gRPC: typed and familiar, but introduces port management and a larger local attack surface.
- Ad-hoc JSON over Unix domain socket: debuggable, but requires custom framing, parsing, and API discipline.
- Tonic gRPC over Unix domain socket: keeps the daemon local and permission-scoped while relying on mature RPC tooling.

## Consequences

The daemon is installed as a per-user LaunchAgent by default and should not require admin privileges. GUI and CLI may write user intent such as tracking, labels, aliases, SSH usernames, and merge/split corrections, but discovery observations, endpoint reachability, Tailscale state, and last-seen/status updates are daemon-owned. The CLI must still be able to read SQLite and report stale or daemon-down state when the daemon is unavailable.
