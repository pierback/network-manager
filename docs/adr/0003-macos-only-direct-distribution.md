# Target macOS-only direct distribution

Network Manager will target macOS only and will not preserve cross-platform compatibility unless it is essentially free. Distribution will be direct/notarized app packaging with an installed CLI helper and LaunchAgent support, rather than App Store or Homebrew-first distribution, because the product depends on macOS-specific local networking, daemon, UI, and shell-integration behavior and does not need cross-OS portability.

## Considered Options

- Cross-platform Rust core: theoretically possible, but adds abstraction work without product value right now.
- App Store distribution: increases sandboxing and review constraints for a local network discovery tool.
- Homebrew-first distribution: useful for CLI tools, but this product is primarily a macOS app with daemon and UI installation concerns.

## Consequences

The implementation may use macOS-specific APIs and paths directly where that improves the app. The CLI remains first-class, but packaging is centered on the macOS app install flow rather than Homebrew.
