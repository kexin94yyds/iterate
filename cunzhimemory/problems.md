# 问题解决经验记录

记录开发过程中遇到的典型问题和解决方案，供将来参考。

---

## 问题 1：MCP 弹窗空白

**日期**: 2024-12-13

**症状**:
- MCP 调用时弹窗显示空白
- 控制台显示 `Failed to load resource: Could not connect to the server. http://localhost:5176/`

**原因**:
1. 使用 `cargo build --release` 而不是 `npm run tauri build`
2. `cargo build` 不会嵌入前端资源，只有 Tauri 构建流程才会

**解决方案**:
```bash
npm run tauri build
```

**回归检查**:
- 构建后运行 `./target/release/iterate`，确认弹窗正常显示内容
- 不依赖开发服务器（localhost:5176）

---

## 问题 2：前端样式修改不生效

**日期**: 2024-12-13

**症状**:
- 修改了前端代码（如主题颜色）
- 但 MCP 弹窗还是显示旧样式

**原因**:
1. 没有重新构建前端
2. 没有重新编译 Rust
3. MCP 配置指向的是旧版本二进制

**解决方案**:
```bash
./deploy.sh
```

**回归检查**:
- 修改前端后运行 `deploy.sh`
- 确认 `/Applications/iterate.app` 时间戳更新
- 测试 MCP 弹窗显示新样式

---

## 问题 3：GUI 标题栏只显示项目名称，无法显示完整路径

**Issue ID**: CUNZHI-003

**日期**: 2024-12-14

**状态**: verified

**症状**:
- GUI 标题栏显示 `iterate / cunzhi`（只有项目名称）
- Dock 栏悬停 tooltip 只显示 `iterate`
- 用户无法区分同名但不同路径的项目

**原因**:
1. `PopupHeader.vue` 中 `displayProjectName` 只取路径最后一部分
2. 窗口标题是静态的 `iterate`，未动态设置

**解决方案**:
1. 修改 `PopupHeader.vue`，使用 `displayProjectPath` 显示完整路径
2. 在 `useMcpHandler.ts` 的 `showMcpDialog` 中动态设置窗口标题
3. 在 `tauri.conf.json` 添加 `core:window:allow-set-title` 权限

**修改文件**:
- `src/frontend/components/popup/PopupHeader.vue`
- `src/frontend/composables/useMcpHandler.ts`
- `tauri.conf.json`

**回归检查**:
- 脚本: `regression_cunzhi_003.sh`
- 测试文件: `test_project_path_display.json`
- 执行方式: `./regression_cunzhi_003.sh`
- 通过条件: 用户确认标题栏和 Dock 都正确显示完整路径

---
