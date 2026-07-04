#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
app_name="Network Manager"
dist_dir="$repo_root/dist"
app_dir="$dist_dir/$app_name.app"
contents_dir="$app_dir/Contents"
macos_dir="$contents_dir/MacOS"
resources_dir="$contents_dir/Resources"
icon_svg="$repo_root/crates/ui/assets/app-icon.svg"
iconset_dir="$dist_dir/NetworkManager.iconset"
icon_file="NetworkManager.icns"

cd "$repo_root"

cargo build --release -p network-manager-ui -p network-manager-cli -p network-manager-daemon --locked

rm -rf "$app_dir"
mkdir -p "$macos_dir" "$resources_dir"

if command -v rsvg-convert >/dev/null 2>&1 && command -v iconutil >/dev/null 2>&1; then
  rm -rf "$iconset_dir"
  mkdir -p "$iconset_dir"
  rsvg-convert -w 16 -h 16 "$icon_svg" -o "$iconset_dir/icon_16x16.png"
  rsvg-convert -w 32 -h 32 "$icon_svg" -o "$iconset_dir/icon_16x16@2x.png"
  rsvg-convert -w 32 -h 32 "$icon_svg" -o "$iconset_dir/icon_32x32.png"
  rsvg-convert -w 64 -h 64 "$icon_svg" -o "$iconset_dir/icon_32x32@2x.png"
  rsvg-convert -w 128 -h 128 "$icon_svg" -o "$iconset_dir/icon_128x128.png"
  rsvg-convert -w 256 -h 256 "$icon_svg" -o "$iconset_dir/icon_128x128@2x.png"
  rsvg-convert -w 256 -h 256 "$icon_svg" -o "$iconset_dir/icon_256x256.png"
  rsvg-convert -w 512 -h 512 "$icon_svg" -o "$iconset_dir/icon_256x256@2x.png"
  rsvg-convert -w 512 -h 512 "$icon_svg" -o "$iconset_dir/icon_512x512.png"
  rsvg-convert -w 1024 -h 1024 "$icon_svg" -o "$iconset_dir/icon_512x512@2x.png"
  iconutil -c icns "$iconset_dir" -o "$resources_dir/$icon_file"
  rm -rf "$iconset_dir"
else
  printf 'warning: rsvg-convert or iconutil is unavailable; packaging without an app icon\n' >&2
fi

cp target/release/network-manager-ui "$macos_dir/$app_name"
cp target/release/network-manager "$resources_dir/network-manager"
cp target/release/network-manager-daemon "$resources_dir/network-manager-daemon"
chmod +x "$macos_dir/$app_name" "$resources_dir/network-manager" "$resources_dir/network-manager-daemon"

cat > "$contents_dir/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleDevelopmentRegion</key>
  <string>en</string>
  <key>CFBundleDisplayName</key>
  <string>Network Manager</string>
  <key>CFBundleExecutable</key>
  <string>Network Manager</string>
  <key>CFBundleIdentifier</key>
  <string>com.network-manager.app</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundleIconFile</key>
  <string>NetworkManager</string>
  <key>CFBundleName</key>
  <string>Network Manager</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleShortVersionString</key>
  <string>0.1.0</string>
  <key>CFBundleVersion</key>
  <string>0.1.0</string>
  <key>LSMinimumSystemVersion</key>
  <string>13.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
</dict>
</plist>
PLIST

if command -v codesign >/dev/null 2>&1; then
  codesign --force --deep --sign - "$app_dir"
fi

printf 'Packaged %s\n' "$app_dir"
printf 'Open with: open %q\n' "$app_dir"
printf 'Install daemon from the app Settings screen or run:\n  %q daemon install --daemon-path %q --force --load\n' \
  "$resources_dir/network-manager" "$resources_dir/network-manager-daemon"
