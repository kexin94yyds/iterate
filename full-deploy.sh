#!/bin/bash

# 🚀 全功能部署脚本 - iterate (寸止)
# 功能：关闭测试进程 → 备份 → 构建 → 安装 → 启动

set -e

APP_NAME="iterate"
APP_PATH="/Applications/${APP_NAME}.app"
BACKUP_PATH="/Applications/${APP_NAME}.app.bak"
PROJECT_DIR="/Users/apple/cunzhi/cunzhi"
BUILD_OUTPUT="target/release/bundle/macos/${APP_NAME}.app"

cd "$PROJECT_DIR"

echo "=========================================="
echo "🚀 全功能部署脚本 - ${APP_NAME}"
echo "=========================================="
echo ""

# Step 1: 关闭运行中的进程
echo "📍 Step 1: 关闭运行中的进程..."
if pgrep -x "$APP_NAME" > /dev/null 2>&1; then
    echo "   发现运行中的 ${APP_NAME}，正在关闭..."
    pkill -x "$APP_NAME" || true
    sleep 1
    echo "   ✅ 已关闭"
else
    echo "   ✅ 无运行中的进程"
fi

# 同时关闭可能的开发服务器
if pgrep -f "tauri" > /dev/null 2>&1; then
    echo "   发现 tauri 开发进程，正在关闭..."
    pkill -f "tauri" || true
    sleep 1
    echo "   ✅ 已关闭开发进程"
fi

# Step 2: 备份现有版本
echo ""
echo "📍 Step 2: 备份现有版本..."
if [ -d "$APP_PATH" ]; then
    # 删除旧备份
    if [ -d "$BACKUP_PATH" ]; then
        rm -rf "$BACKUP_PATH"
        echo "   已删除旧备份"
    fi
    # 备份当前版本
    mv "$APP_PATH" "$BACKUP_PATH"
    echo "   ✅ 已备份到 ${BACKUP_PATH}"
else
    echo "   ⚠️ 无现有版本需要备份"
fi

# Step 3: 构建新版本
echo ""
echo "📍 Step 3: 构建新版本..."
echo "   这可能需要几分钟..."
npm run tauri:build
echo "   ✅ 构建完成"

# Step 4: 安装到 /Applications
echo ""
echo "📍 Step 4: 安装到 /Applications..."
if [ -d "$BUILD_OUTPUT" ]; then
    cp -R "$BUILD_OUTPUT" "$APP_PATH"
    echo "   ✅ 已安装到 ${APP_PATH}"
else
    echo "   ❌ 构建输出不存在: ${BUILD_OUTPUT}"
    exit 1
fi

# Step 5: 启动应用
echo ""
echo "📍 Step 5: 启动应用..."
open "$APP_PATH"
echo "   ✅ 应用已启动"

# 完成
echo ""
echo "=========================================="
echo "✅ 部署完成！"
echo "=========================================="
echo ""
echo "📋 摘要："
echo "   - 新版本: ${APP_PATH}"
echo "   - 备份位置: ${BACKUP_PATH}"
echo ""
echo "💡 如需回滚，运行："
echo "   rm -rf ${APP_PATH} && mv ${BACKUP_PATH} ${APP_PATH}"
echo ""
