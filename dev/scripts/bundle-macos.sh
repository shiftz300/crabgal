#!/usr/bin/env bash
set -euo pipefail

project="${1:-projects/test-project}"
name="${2:-crabgal}"
root="$(cd "$(dirname "$0")/../.." && pwd)"
bundle="$root/target/bundle/macos/$name.app"

cd "$root"
cargo build --release
rm -rf "$bundle"
mkdir -p "$bundle/Contents/MacOS" "$bundle/Contents/Resources/project"
cp target/release/crabgal "$bundle/Contents/MacOS/crabgal"
cp -R "$project"/. "$bundle/Contents/Resources/project/"

sed "s/__NAME__/$name/g" > "$bundle/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>launch</string>
<key>CFBundleIdentifier</key><string>dev.crabgal.__NAME__</string>
<key>CFBundleName</key><string>__NAME__</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>0.6.0</string>
</dict></plist>
PLIST

cat > "$bundle/Contents/MacOS/launch" <<'LAUNCH'
#!/usr/bin/env bash
launcher_dir="$(cd "$(dirname "$0")" && pwd)"
cd "$launcher_dir/../Resources/project"
exec "$launcher_dir/crabgal" .
LAUNCH
chmod +x "$bundle/Contents/MacOS/crabgal" "$bundle/Contents/MacOS/launch"
echo "$bundle"
