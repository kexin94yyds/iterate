#!/bin/bash

# CunZhi 更新脚本
# 一键编译并安装到本地应用

set -e

PROJECT_DIR="$(cd "$(dirname "$0")" && pwd)"
APP_NAME="iterate"

# 预先获取 sudo 权限（弹出 GUI 密码对话框）
acquire_sudo() {
    # 检查是否已有 sudo 权限
    if sudo -n true 2>/dev/null; then
        return 0
    fi
    
    # 使用 osascript 弹出密码对话框
    osascript -e 'do shell script "echo" with administrator privileges' 2>/dev/null
    if [ $? -ne 0 ]; then
        echo "❌ 未获取管理员权限，取消更新"
        exit 1
    fi
}
APP_PATH="/Applications/${APP_NAME}.app"
MCP_BIN_NAME="寸止"
MCP_INSTALL_PATH="/usr/local/bin/cunzhi-mcp"

echo "🔧 CunZhi 更新脚本"
echo "=================="
echo ""

# 预先获取 sudo 权限
echo "🔐 获取管理员权限..."
acquire_sudo
echo "✅ 已获取权限"
echo ""

# 1. 编译 Release 版本
echo "📦 步骤 1/4: 编译 Release 版本..."
cd "$PROJECT_DIR"
cargo build --release
echo "✅ 编译完成"
echo ""

# 2. 构建 Tauri 应用（跳过签名）
echo "📱 步骤 2/4: 构建 Tauri 应用（跳过签名）..."
cd "$PROJECT_DIR"
# 设置环境变量跳过签名
export APPLE_SIGNING_IDENTITY="-"
export TAURI_SIGNING_PRIVATE_KEY=""
npm run tauri build -- --no-bundle 2>/dev/null || cargo tauri build --no-bundle 2>/dev/null || true

# 如果 no-bundle 不支持，直接构建
if [ ! -d "target/release/bundle/macos/${APP_NAME}.app" ]; then
    echo "⏳ 正在进行完整构建..."
    npm run tauri build 2>/dev/null || cargo tauri build 2>/dev/null || {
        echo "⚠️ Tauri 构建跳过，使用现有的 bundle"
    }
fi
echo "✅ 构建完成"
echo ""

# 3. 安装应用到 /Applications
echo "📲 步骤 3/4: 安装应用到 /Applications..."
BUNDLE_APP="$PROJECT_DIR/target/release/bundle/macos/${APP_NAME}.app"

if [ -d "$BUNDLE_APP" ]; then
    # 关闭正在运行的应用
    pkill -f "$APP_NAME" 2>/dev/null || true
    sleep 1
    
    # 删除旧应用并复制新应用
    echo "   删除旧应用: $APP_PATH"
    sudo rm -rf "$APP_PATH"
    echo "   复制新应用: $BUNDLE_APP -> $APP_PATH"
    sudo cp -R "$BUNDLE_APP" "$APP_PATH"
    
    # 同步最新的二进制文件到应用内（避免 Tauri 缓存问题）
    echo "   同步主程序到应用内..."
    sudo rm "$APP_PATH/Contents/MacOS/$APP_NAME"
    sudo cp "$PROJECT_DIR/target/release/$APP_NAME" "$APP_PATH/Contents/MacOS/$APP_NAME"
    echo "   同步 MCP 服务器到应用内..."
    sudo rm "$APP_PATH/Contents/MacOS/$MCP_BIN_NAME"
    sudo cp "$PROJECT_DIR/target/release/$MCP_BIN_NAME" "$APP_PATH/Contents/MacOS/$MCP_BIN_NAME"
    
    # 使用 ad-hoc 签名（允许应用运行）
    echo "   签名应用..."
    sudo codesign --force --deep --sign - "$APP_PATH"
    sudo xattr -cr "$APP_PATH" 2>/dev/null || true
    
    echo "✅ 应用已安装到 $APP_PATH"
else
    echo "⚠️ 未找到构建的应用: $BUNDLE_APP"
    echo "   跳过应用安装"
fi
echo ""

# 4. 安装 MCP 服务器
echo "🔌 步骤 4/4: 安装 MCP 服务器..."
MCP_BIN="$PROJECT_DIR/target/release/$MCP_BIN_NAME"

if [ -f "$MCP_BIN" ]; then
    echo "   复制 MCP 服务器: $MCP_BIN -> $MCP_INSTALL_PATH"
    sudo cp "$MCP_BIN" "$MCP_INSTALL_PATH"
    sudo chmod +x "$MCP_INSTALL_PATH"
    echo "✅ MCP 服务器已安装到 $MCP_INSTALL_PATH"
else
    echo "⚠️ 未找到 MCP 服务器: $MCP_BIN"
fi
echo ""

# 完成
echo "=================="
echo "🎉 更新完成！"
echo ""
echo "提示："
echo "  - 如果更新了 MCP 工具，请重启 Windsurf"
echo "  - 如果更新了桌面应用，可以直接打开 /Applications/${APP_NAME}.app"
echo ""
