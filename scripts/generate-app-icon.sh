#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
icon_svg="$repo_root/crates/ui/assets/app-icon.svg"
icon_file="$repo_root/crates/ui/assets/NetworkManager.icns"
work_dir="$(mktemp -d "$repo_root/crates/ui/assets/.icon-build.XXXXXX")"
iconset_dir="$work_dir/NetworkManager.iconset"
generated_icon="$work_dir/NetworkManager.icns"

trap 'rm -rf "$work_dir"' EXIT

if ! command -v rsvg-convert >/dev/null 2>&1; then
  printf 'error: rsvg-convert is required to regenerate the app icon\n' >&2
  exit 1
fi
if ! command -v iconutil >/dev/null 2>&1; then
  printf 'error: iconutil is required to regenerate the app icon\n' >&2
  exit 1
fi

mkdir -p "$iconset_dir"

render_icon() {
  local size="$1"
  local name="$2"
  rsvg-convert -w "$size" -h "$size" "$icon_svg" -o "$iconset_dir/$name"
}

render_icon 16 icon_16x16.png
render_icon 32 icon_32x32.png
cp "$iconset_dir/icon_32x32.png" "$iconset_dir/icon_16x16@2x.png"
render_icon 64 icon_32x32@2x.png
render_icon 128 icon_128x128.png
render_icon 256 icon_256x256.png
cp "$iconset_dir/icon_256x256.png" "$iconset_dir/icon_128x128@2x.png"
render_icon 512 icon_512x512.png
cp "$iconset_dir/icon_512x512.png" "$iconset_dir/icon_256x256@2x.png"
render_icon 1024 icon_512x512@2x.png

iconutil -c icns "$iconset_dir" -o "$generated_icon"
test -s "$generated_icon"
mv "$generated_icon" "$icon_file"

printf 'Generated %s\n' "$icon_file"
