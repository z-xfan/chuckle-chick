# REQ-0006：Windows x64 自动构建

## 基本信息

- 状态：待验收
- 创建日期：2026-07-21
- 最后更新：2026-07-21
- 负责人：待定

## 背景

项目目前主要在 macOS 上开发和验证，需要通过 GitHub Actions 的 Windows Runner 生成可下载的 Windows x64 安装包，为后续 Windows 实机验收提供产物。

## 目标

- 在 GitHub Actions 中手动触发 Windows x64 构建。
- 使用项目现有 Tauri 2 与 NSIS 配置生成安装程序。
- 将安装程序作为本次 Actions 运行的 Artifact 提供下载。

## 范围

### 包含

- `windows-latest` 标准 Runner。
- `x86_64-pc-windows-msvc` Rust 目标。
- pnpm 冻结锁文件安装、前端构建和 Tauri NSIS 打包。
- 手动触发及构建产物上传。

### 不包含

- Windows 代码签名证书与 SmartScreen 信誉。
- 自动创建 GitHub Release、自动更新或自动发布。
- Windows ARM64、MSI、macOS 和 Linux 安装包。

## 需求说明

- 工作流仅通过 `workflow_dispatch` 手动触发，避免普通提交重复消耗构建资源。
- Node.js 使用 22，pnpm 使用项目声明的 11.9.0，Rust 使用 stable。
- 构建目标固定为 `x86_64-pc-windows-msvc`，安装包格式固定为 NSIS。
- 构建成功后上传命名清晰的 workflow Artifact。

## 验收标准

- [x] 工作流可以从 GitHub Actions 页面手动触发。
- [x] Windows Runner 完成依赖安装、测试和 Tauri 构建。
- [x] Actions 页面提供 Windows x64 NSIS Artifact 下载。
- [x] Artifact 中包含 `.exe` 安装程序。
- [x] 工作流不创建 Release，不要求签名密钥。
- [ ] Windows 安装版可以读取随包发布的宠物 JSON 与图片资源并正常显示宠物。

## 实施记录

- 2026-07-21：新增手动触发的 `Build Windows x64` 工作流，使用 `windows-latest`、Node.js 22、pnpm 11.9.0 和 Rust stable。
- 2026-07-21：构建前运行前端测试，随后通过 Tauri 官方 Action 生成 `x86_64-pc-windows-msvc` NSIS 安装包并上传 workflow Artifact。
- 2026-07-21：首次运行后将 GitHub 官方基础 Actions 升级到 Node.js 24 运行时版本，避免弃用警告。
- 2026-07-21：Windows 实机运行发现 CSP 未允许同源 `fetch`，导致宠物 JSON 资源加载失败；补充安装版资源加载修复。
- 2026-07-21：Windows 安装产物实机运行发现 PixiJS 在 Tauri 严格 CSP 下触发 `unsafe-eval` 报错；在应用入口预加载 `pixi.js/unsafe-eval` 静态回退模块，避免放宽 Tauri CSP。

## 验证记录

- 2026-07-21：GitHub Actions 运行 `29818774620` 成功，Windows x64 NSIS Job 用时 6 分 44 秒。
- 2026-07-21：Artifact `ChuckleChick-windows-x64-nsis` 上传成功，归档大小约 4.5 MB。
- 2026-07-21：下载并核对 Artifact，包含 `ChuckleChick_0.1.0_x64-setup.exe`。
- 2026-07-21：升级基础 Actions 后再次运行 `29819324504`，6 分 41 秒成功完成且无 Node.js 20 弃用警告，Artifact 再次生成。
- 2026-07-21：CSP 的 `connect-src` 已加入 `'self'`；前端 11 项测试和 Rust 4 项测试通过，生产构建成功且确认三份宠物资源均进入 `dist`。等待新的 Windows 安装包实机验收。
- 2026-07-21：本地执行前端测试与生产构建，确认 `pixi.js/unsafe-eval` 可被 Vite 打包解析；等待新的 GitHub Actions Windows Artifact 实机验收。

## 变更记录

| 日期 | 变更内容 |
| --- | --- |
| 2026-07-21 | 用户确认采用 GitHub Actions Windows Runner 自动构建 Windows x64 安装包 |
| 2026-07-21 | 补充 Windows 安装版必须能够正常加载随包宠物资源的验收要求 |
| 2026-07-21 | 补充 PixiJS 在 Tauri 严格 CSP 下不得触发 `unsafe-eval` 运行时报错的修复记录 |
