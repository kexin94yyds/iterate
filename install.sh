#!/bin/bash

# 寸止 MCP 工具 - 最简化安装脚本
# 只需构建两个CLI工具即可运行MCP

set -e

echo "🚀 安装 寸止 MCP 工具..."

# 检查必要工具
if ! command -v "cargo" &> /dev/null; then
    echo "❌ 请先安装 cargo"
    exit 1
fi

# 构建
if command -v "pnpm" &> /dev/null; then
    echo "📦 构建前端资源..."
    pnpm build
else
    echo "⚠️ 未检测到 pnpm，跳过前端构建（仅安装 iterate 可执行文件用于 MCP）"
fi

echo "🔨 构建 CLI 工具..."
cargo build --release

# 检查构建结果
if [[ ! -f "target/release/iterate" ]] || [[ ! -f "target/release/寸止" ]]; then
    echo "❌ 构建失败"
    exit 1
fi

# 安装到用户目录
BIN_DIR="$HOME/.local/bin"
mkdir -p "$BIN_DIR"

# 安装 iterate（GUI 程序）
cp "target/release/iterate" "$BIN_DIR/"
chmod +x "$BIN_DIR/iterate"

# 安装 寸止（MCP server，独立二进制，不是软链接）
cp "target/release/寸止" "$BIN_DIR/"
chmod +x "$BIN_DIR/寸止"

# 兼容旧命令名：等一下 指向 iterate
ln -sf "$BIN_DIR/iterate" "$BIN_DIR/等一下"

echo "✅ 安装完成！CLI 工具已安装到 $BIN_DIR"

# 检查PATH
if [[ ":$PATH:" != *":$BIN_DIR:"* ]]; then
    echo ""
    echo "💡 请将以下内容添加到 ~/.bashrc 或 ~/.zshrc:"
    echo "export PATH=\"\$PATH:$BIN_DIR\""
    echo "然后运行: source ~/.bashrc"
fi

echo ""
echo "📋 使用方法："
echo "  iterate     - 启动 GUI 界面"
echo "  寸止        - 启动 MCP Server（stdio 模式）"
echo "  等一下      - 兼容旧命令名（指向 iterate）"
echo ""
echo "📝 MCP 客户端配置："
echo '{"mcpServers": {"cunzhi": {"command": "寸止"}}}'
