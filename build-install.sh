#!/bin/bash

# 快速构建并安装 iterate 应用
set -e

cd /Users/apple/cunzhi/cunzhi

echo "🔨 开始构建..."
npm run tauri:build

echo "📦 安装到 /Applications/..."
rm -rf /Applications/iterate.app
cp -r target/release/bundle/macos/iterate.app /Applications/

echo "🚀 启动应用..."
open /Applications/iterate.app

echo "✅ 完成！"
