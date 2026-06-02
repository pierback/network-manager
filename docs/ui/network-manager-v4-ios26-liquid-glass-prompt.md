# Network Manager UI V4 — iOS 26 Liquid Glass / Cursor-like Monochrome

Iterate the existing design toward a modern Apple iOS 26-style Liquid Glass look, but in a macOS desktop utility context.

## Correction from user

The desired direction is NOT old graphite macOS and NOT flashy neon glass. The target is:

- new Apple iOS 26 Liquid Glass feel
- more monochrome
- Cursor-like calm premium UI
- translucent glass surfaces with refraction/highlights
- black/white/gray dominant palette
- very restrained accent color usage
- precise component spacing

## Visual direction

- Monochrome, premium, minimal.
- Cursor-like: dark neutral base, crisp panels, subtle separators, confident typography, low saturation.
- Liquid Glass: layered translucent panes, soft frosted blur, thin bright edge highlights, subtle specular highlights.
- Avoid colorful glows, neon gradients, blue-heavy dashboards.
- Accent colors only for semantic status dots or selected state.
- Use more white/black contrast inside glass rather than saturated color.
- Keep shapes soft but not bubbly: 14–22px radii for panels, 8–12px for controls.
- Prefer compact native utility density.

## Palette guidance

- Background: near-black / deep charcoal.
- Glass panes: semi-transparent charcoal, smoke, white tint overlays.
- Borders: 1px white at low opacity, plus occasional top-edge highlight.
- Text: white, light gray, muted gray.
- Accent: minimal icy blue or white selection, not bright blue.
- Status: small green/red/yellow/gray dots only.

## Spacing / layout requirements

- Strict 8px grid.
- Fix all component spacing and alignment.
- Columns must align mathematically.
- No clipped controls.
- No overfull rows.
- Menu popover must feel especially polished.

## Screens to produce

Create new V4 artboards:

1. Dashboard V4
   - Monochrome Liquid Glass window.
   - Dashboard rows/cards should feel like layered glass slabs.
   - Summary metrics should be subtle, not colorful.

2. Discovery V4
   - Quiet filter chips and search.
   - Monochrome rows; source badges reduced to low-contrast pills.
   - Possible match callout as a transparent glass note.

3. Device Detail V4
   - Inspector with glass cards.
   - Endpoint groups clearly separated.
   - SSH target reasoning as a refined system note.

4. Quick Access Popover V4
   - Most important artboard.
   - Looks like a real modern Apple menu-bar liquid-glass popover.
   - Compact rows, hover-like subtle glass action buttons, tiny status dots.

5. Settings V4
   - Monochrome grouped controls.
   - Calm, Cursor-like utility settings.

## Preserve semantics

- Dashboard shows Tracked Devices only.
- Discovery shows all Discovered Devices.
- LAN/Tailscale/SSH statuses separate.
- SSH target uses Network Proximity: LAN when locally reachable, Tailscale otherwise.
- Device Alias remains CLI-friendly.

## Deliverable

Create a new V4 Pencil file from the input. Export a preview PNG and individual artboard previews if possible.
