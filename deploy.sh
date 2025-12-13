#!/bin/bash
# 快速部署脚本 - 构建并安装到本地

set -e

echo "🔨 开始构建..."
npm run tauri build

echo "📦 备份旧版本..."
if [ -d "/Applications/iterate.app" ]; then
    rm -rf /Applications/iterate.app.bak
    mv /Applications/iterate.app /Applications/iterate.app.bak
fi

echo "🚀 安装新版本..."
cp -R target/release/bundle/macos/iterate.app /Applications/

echo "✅ 部署完成！"
echo "   - 新版本: /Applications/iterate.app"
echo "   - 备份: /Applications/iterate.app.bak"
