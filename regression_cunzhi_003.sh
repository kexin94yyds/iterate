#!/bin/bash

# 回归检查脚本 - CUNZHI-003
# 验证 GUI 标题栏显示完整项目路径
# Issue: GUI 标题栏只显示项目名称，无法显示完整路径

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CLI_PATH="$PROJECT_ROOT/target/release"
TEST_FILE="$PROJECT_ROOT/test_project_path_display.json"

echo -e "${BLUE}================================================${NC}"
echo -e "${BLUE}  回归检查 CUNZHI-003: 标题栏显示完整项目路径${NC}"
echo -e "${BLUE}================================================${NC}"
echo ""

# 检查测试文件
if [[ ! -f "$TEST_FILE" ]]; then
    echo -e "${RED}❌ 测试文件不存在: $TEST_FILE${NC}"
    exit 1
fi

# 检查 CLI 工具
if [[ ! -f "$CLI_PATH/iterate" ]]; then
    echo -e "${RED}❌ iterate 不存在，请先构建: npm run build && cargo build --release${NC}"
    exit 1
fi

echo -e "${YELLOW}📋 检查项:${NC}"
echo -e "  1. GUI 标题栏应显示完整路径（如 /Users/apple/cunzhi）"
echo -e "  2. Dock 栏悬停应显示 iterate - /Users/apple/cunzhi"
echo ""

echo -e "${GREEN}🚀 启动测试弹窗...${NC}"
echo -e "${BLUE}执行: $CLI_PATH/iterate --mcp-request $TEST_FILE${NC}"
echo ""

# 运行测试
RESULT=$("$CLI_PATH/iterate" --mcp-request "$TEST_FILE" 2>&1) || true

echo ""
echo -e "${YELLOW}📝 用户响应: ${NC}$RESULT"
echo ""

# 检查结果
if [[ "$RESULT" == *"✅"* ]]; then
    echo -e "${GREEN}================================================${NC}"
    echo -e "${GREEN}  ✅ 回归检查通过 - CUNZHI-003${NC}"
    echo -e "${GREEN}================================================${NC}"
    exit 0
else
    echo -e "${RED}================================================${NC}"
    echo -e "${RED}  ❌ 回归检查失败 - CUNZHI-003${NC}"
    echo -e "${RED}  用户响应: $RESULT${NC}"
    echo -e "${RED}================================================${NC}"
    exit 1
fi
