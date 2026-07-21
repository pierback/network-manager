#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
app_name="Network Manager"
dist_dir="$repo_root/dist"
app_dir="$dist_dir/$app_name.app"
contents_dir="$app_dir/Contents"
macos_dir="$contents_dir/MacOS"
resources_dir="$contents_dir/Resources"
icon_file="NetworkManager.icns"
# Commit the generated icon so packaging does not depend on SVG conversion tools.
icon_source="$repo_root/crates/ui/assets/$icon_file"

cd "$repo_root"

test -s "$icon_source"
cargo build --release -p network-manager-ui -p network-manager-cli -p network-manager-daemon --locked

rm -rf "$app_dir"
mkdir -p "$macos_dir" "$resources_dir"

cp target/release/network-manager-ui "$macos_dir/$app_name"
cp target/release/network-manager "$resources_dir/network-manager"
cp target/release/network-manager-daemon "$resources_dir/network-manager-daemon"
cp "$icon_source" "$resources_dir/$icon_file"
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
