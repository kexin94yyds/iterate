#!/bin/bash

# 窗口切换测试脚本
# 用于测试 CUNZHI-004: 窗口切换器点击第二行无效的问题

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

CLI_PATH="/Applications/iterate.app/Contents/MacOS/寸止"
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo -e "${BLUE}================================================${NC}"
echo -e "${BLUE}  窗口切换测试 - CUNZHI-004${NC}"
echo -e "${BLUE}================================================${NC}"
echo ""

# 检查 CLI 工具
if [[ ! -f "$CLI_PATH" ]]; then
    echo -e "${RED}❌ CLI 工具不存在: $CLI_PATH${NC}"
    exit 1
fi

# 清理旧的注册表
rm -f "$TMPDIR/iterate_windows.json" 2>/dev/null || true

# 创建测试 JSON 文件
cat > /tmp/test_win_a.json << 'EOF'
{
  "id": "test-win-a",
  "message": "# 窗口 A\n\n这是测试窗口 A。\n\n按 **Tab** 打开窗口切换器，然后点击 **窗口 B** 测试。",
  "predefined_options": ["确认"],
  "is_markdown": true,
  "project_path": "/Users/apple/test-project-A"
}
EOF

cat > /tmp/test_win_b.json << 'EOF'
{
  "id": "test-win-b",
  "message": "# 窗口 B\n\n这是测试窗口 B。\n\n按 **Tab** 打开窗口切换器，然后点击 **窗口 A** 测试。",
  "predefined_options": ["确认"],
  "is_markdown": true,
  "project_path": "/Users/apple/test-project-B"
}
EOF

echo -e "${GREEN}🚀 启动窗口 A...${NC}"
"$CLI_PATH" --mcp-request /tmp/test_win_a.json &
PID_A=$!
echo -e "${BLUE}   PID: $PID_A${NC}"

sleep 2

echo -e "${GREEN}🚀 启动窗口 B...${NC}"
"$CLI_PATH" --mcp-request /tmp/test_win_b.json &
PID_B=$!
echo -e "${BLUE}   PID: $PID_B${NC}"

sleep 2

echo ""
echo -e "${YELLOW}📋 测试步骤:${NC}"
echo -e "  1. 在任一窗口按 ${GREEN}Tab${NC} 打开窗口切换器"
echo -e "  2. 点击 ${GREEN}第二行${NC} 的窗口"
echo -e "  3. 验证是否切换到了正确的窗口"
echo ""
echo -e "${BLUE}按 Ctrl+C 结束测试${NC}"

# 等待进程结束
wait $PID_A $PID_B 2>/dev/null || true

echo ""
echo -e "${GREEN}✅ 测试结束${NC}"
