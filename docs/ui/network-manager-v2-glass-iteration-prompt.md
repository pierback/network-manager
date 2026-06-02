# Network Manager UI V2 Iteration Prompt

Iterate the existing Network Manager Pencil design into a more polished, glassy, native macOS productivity app.

## Role

Act like a brutally honest senior macOS product design director. First mentally critique the current design, then apply the fixes directly in the Pencil file. Keep what works, but improve visual hierarchy, spacing, placement, density, and macOS feel.

## Must preserve

- Same product concepts and screens:
  - Dashboard
  - Discovery
  - Device Detail / Inspector
  - Quick Access / menu-bar popover
  - Settings
- Same domain semantics:
  - Tracked Devices appear on Dashboard
  - Discovery contains all observable devices
  - LAN / Tailscale / SSH statuses are separate
  - Network Proximity influences SSH target reasoning
  - Status colors: green online, red offline, gray unknown, yellow stale/degraded
- Same data examples are fine, but polish their presentation.

## Desired visual direction

Make it feel like a premium native macOS app, not a flat web dashboard:

- Frosted-glass / translucent material surfaces.
- Layered depth: background glow, panes, cards, inspector, popover.
- macOS-style sidebar + toolbar composition.
- Softer rounded corners, subtle borders, inner highlights, ambient shadows.
- Compact but breathable spacing.
- Better status hierarchy: overall status should be immediately scannable; endpoint details should not visually compete.
- Dark mode first, but not pure black; use tinted graphite/navy glass.
- Avoid cheesy neon or overdone glassmorphism.

## Concrete critique targets

1. **Dashboard**
   - Improve top toolbar/titlebar placement.
   - Make dashboard rows/cards less flat and more Mac-like.
   - Improve column spacing so statuses are easier to scan.
   - Make the preferred SSH target visually secondary but clear.
   - Add a subtle summary strip: tracked count, online count, Tailscale service status, last scan.

2. **Discovery**
   - Make filters less web-form-like; use compact macOS segmented/filter chips.
   - Better separation between search/filter area and result rows.
   - Make source badges quieter; they should not overpower device names.
   - Show possible match/merge suggestion as a tasteful inline callout.

3. **Device Detail / Inspector**
   - Make the inspector feel like a native side panel.
   - Put label + alias editing in a clean identity card.
   - Group endpoints with clearer LAN vs Tailscale sections.
   - SSH target reasoning should look like an explanatory system note, not a random text block.
   - Merge/split controls should feel powerful but not dangerous.

4. **Quick Access / Menu Bar Popover**
   - Make this the most polished artboard.
   - It should look like a real macOS menu-bar popover with glass, notch/anchor, compact rows, hover-like action affordances.
   - Prioritize speed: device status + SSH/copy actions.

5. **Settings**
   - Make settings more native: grouped form sections, subtle dividers, toggles, explanatory footnotes.
   - Reduce visual clutter.

## Layout / spacing rules

- Use consistent 8px spacing scale.
- Prefer 16/20/24px section spacing.
- Align columns precisely.
- Avoid cramped text and clipped controls.
- Reduce unnecessary table borders; use spacing, tint, and dividers instead.
- Keep artboards labeled clearly.

## Deliverable

Create a V2 design in the output Pencil file, preserving the original as input only. Export a preview PNG. The result should be visibly more glassy/native/premium than V1.
