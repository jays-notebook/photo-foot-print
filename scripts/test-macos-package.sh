#!/usr/bin/env bash
# Verify the unsigned universal build, add an ad-hoc signature, and archive it.
# This does not provide Developer ID signing or Apple notarization.
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
bundle=${1:-"$repo_root/target/universal-apple-darwin/release/bundle/macos/photo-foot-print.app"}
version=$(node -p 'require(process.argv[1]).version' "$repo_root/package.json")
plist="$bundle/Contents/Info.plist"
plist_read() { /usr/libexec/PlistBuddy -c "Print :$1" "$plist"; }

[[ $(plist_read CFBundleShortVersionString) == "$version" ]]
[[ $(plist_read CFBundleVersion) == "$version" ]]
binary="$bundle/Contents/MacOS/$(plist_read CFBundleExecutable)"
[[ -x "$binary" ]]
architectures=$(/usr/bin/lipo -archs "$binary")
if [[ "$architectures" != "x86_64 arm64" && "$architectures" != "arm64 x86_64" ]]; then
  echo "Expected exactly arm64 and x86_64, found: $architectures" >&2
  exit 1
fi

# A universal binary needs a valid signature on both slices to launch on Apple
# silicon. This local identity supplies integrity, not publisher trust.
/usr/bin/codesign --force --sign - "$bundle"
/usr/bin/codesign --verify --deep --strict --all-architectures "$bundle"

scratch=$(mktemp -d "${TMPDIR:-/tmp}/pfp-macos-package.XXXXXX")
app_pid=''
cleanup() {
  if [[ -n "$app_pid" ]]; then
    kill -KILL "$app_pid" 2>/dev/null || true
    wait "$app_pid" 2>/dev/null || true
  fi
  rm -rf "$scratch"
}
trap cleanup EXIT

# This is a native launch smoke check, not an interactive desktop acceptance test.
"$binary" >"$scratch/launch.log" 2>&1 &
app_pid=$!
sleep 5
if ! kill -0 "$app_pid" 2>/dev/null; then
  cat "$scratch/launch.log" >&2
  echo "The macOS application exited during its launch check" >&2
  exit 1
fi
kill "$app_pid"
sleep 1
kill -KILL "$app_pid" 2>/dev/null || true
wait "$app_pid" 2>/dev/null || true
app_pid=''

artifact_dir="$repo_root/target/release-artifacts"
mkdir -p "$artifact_dir"
archive="$artifact_dir/photo-foot-print_${version}_macos-universal.app.zip"
# Remove a prior archive so ditto cannot retain entries from an older bundle.
rm -f "$archive"
/usr/bin/ditto -c -k --sequesterRsrc --keepParent "$bundle" "$archive"
/usr/bin/ditto -x -k "$archive" "$scratch/unpacked"
unpacked="$scratch/unpacked/$(basename "$bundle")"
/usr/bin/codesign --verify --deep --strict --all-architectures "$unpacked"
unpacked_binary="$unpacked/Contents/MacOS/$(plist_read CFBundleExecutable)"
[[ -x "$unpacked_binary" ]]
cmp "$binary" "$unpacked_binary"
echo "Verified version $version, architectures $architectures, native launch, and archive integrity: $archive"
