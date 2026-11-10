#!/usr/bin/env bash
# 本地极速调试脚本：增量编译 debug 并通过合法的 macOS Bundle 启动
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# 1. 增量编译 debug（修改代码后只需几秒）
cargo build --bin smelt --bin smeltd

# 2. 保证 target/debug/Smelt.app 结构完整
mkdir -p target/debug/Smelt.app/Contents/MacOS target/debug/Smelt.app/Contents/Resources
# macOS WebKit 要求必须是真实文件或硬链接，不能是跳出 Bundle 外的软链接
rm -f target/debug/Smelt.app/Contents/MacOS/smelt target/debug/Smelt.app/Contents/MacOS/smeltd
ln target/debug/smelt target/debug/Smelt.app/Contents/MacOS/smelt 2>/dev/null || cp -f target/debug/smelt target/debug/Smelt.app/Contents/MacOS/smelt
ln target/debug/smeltd target/debug/Smelt.app/Contents/MacOS/smeltd 2>/dev/null || cp -f target/debug/smeltd target/debug/Smelt.app/Contents/MacOS/smeltd
cp -f assets/AppIcon.icns target/debug/Smelt.app/Contents/Resources/AppIcon.icns 2>/dev/null || true

if [ ! -f target/debug/Smelt.app/Contents/Info.plist ]; then
cat > target/debug/Smelt.app/Contents/Info.plist <<'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>CFBundleName</key>
	<string>Smelt</string>
	<key>CFBundleDisplayName</key>
	<string>Smelt (Debug)</string>
	<key>CFBundleIdentifier</key>
	<string>com.zzfn.smelt.debug</string>
	<key>CFBundleExecutable</key>
	<string>smelt</string>
	<key>CFBundleIconFile</key>
	<string>AppIcon</string>
	<key>CFBundlePackageType</key>
	<string>APPL</string>
	<key>DTSDKName</key>
	<string>macosx</string>
	<key>DTPlatformName</key>
	<string>macosx</string>
</dict>
</plist>
EOF
fi

# 对 debug app 做本地即时签名（adhoc 签名只需要 0.1 秒）
codesign -s - --force target/debug/Smelt.app 2>/dev/null || true

# 3. 隔离日常数据（默认使用 /tmp/smelt-dev），避免干扰正在运行的正式版
export SMELT_HOME="${SMELT_HOME:-/tmp/smelt-dev}"
echo "▶ 启动 Smelt Debug (SMELT_HOME=$SMELT_HOME) …"
exec target/debug/Smelt.app/Contents/MacOS/smelt "$@"
