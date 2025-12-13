# 问题排除指南

记录 cunzhi 项目开发过程中遇到的典型问题和解决方案。

---

## 问题 1：MCP 弹窗空白

### 症状
- MCP 调用时弹窗显示空白
- 控制台显示 `Failed to load resource: Could not connect to the server. http://localhost:5176/`

### 原因
1. **debug 模式**：前端资源不会嵌入到二进制中，需要开发服务器运行
2. **cargo build --release**：没有使用 Tauri 的构建流程，前端资源没有正确嵌入

### 解决方案

**方案 A：使用 Tauri 构建（推荐）**

```bash
npm run build
npm run tauri build
```

这会正确嵌入前端资源到二进制中。

**方案 B：使用 debug 模式 + 开发服务器**

```bash
# 1. 启动开发服务器（保持运行）
cd /Users/apple/cunzhi/cunzhi && npm run dev

# 2. 配置 MCP 使用 debug 版本
# ~/.codeium/windsurf/mcp_config.json:
{
  "mcpServers": {
    "cunzhi": {
      "command": "/Users/apple/cunzhi/cunzhi/target/debug/寸止"
    }
  }
}
```

**注意**：`cargo build --release` 不会正确嵌入前端资源，应使用 `npm run tauri build`

---

## 问题 2：前端样式修改不生效

### 症状
- 修改了前端代码（如主题颜色）
- 但 MCP 弹窗还是显示旧样式

### 原因
1. 没有重新编译前端：`npm run build`
2. 没有重新编译 Rust：`cargo build --release`
3. MCP 配置指向的是 Homebrew 安装的旧版本

### 解决方案

**方案 A：使用本地编译版本**

1. 重新构建：
   ```bash
   npm run build
   cargo build --release
   ```

2. 修改 Windsurf MCP 配置指向本地版本：
   ```bash
   # 修改 ~/.codeium/windsurf/mcp_config.json
   {
     "command": "/Users/apple/cunzhi/cunzhi/target/release/寸止"
   }
   ```

3. 在 MCP 面板点 refresh

**方案 B：更新 Homebrew 版本**

需要发布新版本到 Homebrew（涉及更新 formula 和重新打包）。

---

## 问题 3：直接替换 Homebrew 二进制文件失败

### 症状
- 用 `sudo cp` 替换 `/opt/homebrew/Cellar/cunzhi/0.4.0/bin/` 下的文件
- MCP 仍然不工作或显示空白

### 原因
1. Homebrew 的 `寸止` 是旧版本，查找 `iterate` 的逻辑可能不同
2. Homebrew 版本可能有其他依赖关系
3. 需要同时替换 `寸止` 和 `iterate`

### 解决方案
**不推荐直接替换 Homebrew 文件**。建议使用本地编译版本，或发布新版本到 Homebrew。

如果需要恢复 Homebrew 原版本：
```bash
brew reinstall cunzhi
```

---

## 问题 4：二进制文件混淆

### 概念说明

| 二进制 | 作用 | 说明 |
|--------|------|------|
| `iterate` | GUI 程序 | 包含前端资源，显示弹窗界面 |
| `寸止` | MCP server | stdio 模式，处理 MCP 请求，调用 `iterate` 显示弹窗 |
| `等一下` | 兼容命令 | 软链接指向 `iterate` |

### 查找逻辑
`寸止` 调用弹窗时按以下顺序查找 `iterate`：
1. 同目录下的 `iterate`
2. 全局 `iterate` 命令（PATH 中）

---

## 问题 5：Windsurf MCP 配置文件位置

### 正确位置
```
~/.codeium/windsurf/mcp_config.json
```

### 常见错误
- 修改了项目里的 `cunzhi/mcp_config.json`（这只是示例，不被 Windsurf 读取）

---

## 开发流程总结

### 修改前端后的正确流程

```bash
# 1. 构建前端
npm run build

# 2. 编译 Rust（release 模式）
cargo build --release

# 3. 确保 Windsurf MCP 配置指向 release 版本
# ~/.codeium/windsurf/mcp_config.json:
# "command": "<项目路径>/target/release/寸止"

# 4. 在 MCP 面板点 refresh
```

### 开发调试时使用 tauri dev

```bash
npm run tauri dev
```

这会启动开发服务器，支持热更新，方便调试前端样式。

---

## 其他用户配置指南

### 从源码编译安装

如果你不是项目开发者，而是从 GitHub clone 下来使用：

```bash
# 1. 克隆项目
git clone https://github.com/imhuso/cunzhi.git
cd cunzhi

# 2. 安装依赖
pnpm install

# 3. 构建前端
pnpm build

# 4. 编译 Rust
cargo build --release

# 5. 配置 Windsurf MCP
# 编辑 ~/.codeium/windsurf/mcp_config.json：
{
  "mcpServers": {
    "cunzhi": {
      "command": "<你的项目路径>/target/release/寸止",
      "args": []
    }
  }
}

# 6. 在 Windsurf MCP 面板点 refresh
```

### 使用 Homebrew 安装（推荐普通用户）

```bash
# 添加 tap
brew tap imhuso/cunzhi

# 安装
brew install cunzhi

# 配置 Windsurf MCP
# 编辑 ~/.codeium/windsurf/mcp_config.json：
{
  "mcpServers": {
    "cunzhi": {
      "command": "/opt/homebrew/bin/寸止",
      "args": []
    }
  }
}
```

### 使用安装脚本

```bash
# 在项目目录下运行
./install.sh

# 配置 Windsurf MCP
# 编辑 ~/.codeium/windsurf/mcp_config.json：
{
  "mcpServers": {
    "cunzhi": {
      "command": "~/.local/bin/寸止",
      "args": []
    }
  }
}
```

### 配置文件位置说明

| IDE | 配置文件路径 |
|-----|-------------|
| Windsurf | `~/.codeium/windsurf/mcp_config.json` |
| Cursor | `~/.cursor/mcp_config.json`（待确认） |

### 验证安装

配置完成后，在 MCP 面板点 refresh，然后调用任意 cunzhi 工具（如 `zhi`），应该能看到弹窗界面。
