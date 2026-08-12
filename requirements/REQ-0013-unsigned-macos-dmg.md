# REQ-0013：无签名 macOS DMG 安装包

## 基本信息

- 状态：已完成
- 创建日期：2026-08-12
- 最后更新：2026-08-12
- 负责人：待定

## 背景

项目已经支持在 macOS 开发环境运行，但当前 Tauri 默认打包目标固定为 Windows NSIS，无法直接生成供 Mac 用户安装的磁盘映像。现阶段需要先生成一个无需 Apple Developer 证书的 DMG，用于本机或内部测试。

## 目标

- 在当前 Apple Silicon macOS 开发机生成可挂载的 DMG。
- DMG 内包含可拖入“应用程序”目录的 ChuckleChick 应用。
- 构建不依赖 Apple Developer 证书、签名身份或公证凭据。

## 范围

### 包含

- 基于当前 `main` 代码构建 release 版本。
- 目标架构为当前机器的 `aarch64-apple-darwin`。
- 生成 `.app` 与 `.dmg` 本地产物。
- 检查 DMG 文件类型、挂载内容和签名状态。

### 不包含

- Apple Developer ID 签名与 Apple 公证。
- Mac App Store 发布、自动更新或 GitHub Release。
- Intel Mac 或通用二进制包。
- 修改现有 Windows x64 GitHub Actions。

## 需求说明

- 本次使用 Tauri CLI 的 `dmg` 构建参数覆盖默认 NSIS 目标，不改变 Windows 打包配置。
- 不配置签名证书、Apple ID、Team ID 或公证密钥。
- 产物只用于本机或受控测试环境。
- 用户首次打开时可能被 macOS Gatekeeper 阻止，需要在“系统设置 → 隐私与安全性”中手动允许，或在 Finder 中右键选择“打开”。
- 不建议把无签名包作为面向公众的正式版本发布。
- macOS 应用包必须显式包含并声明黄鸡 `icon.icns`；不能仅依赖 `src-tauri/icons` 目录存在资源。

## 验收标准

- [x] release 前端与 Rust 构建成功。
- [x] 生成 Apple Silicon `.app`。
- [x] 生成可识别、可挂载的 `.dmg` 文件。
- [x] DMG 中包含 ChuckleChick 应用和安装引导入口。
- [x] 构建过程不要求 Apple Developer 证书或公证凭据。
- [x] 记录最终产物路径、大小和验证结果。

## 实施记录

- 使用 Tauri CLI 的 `--bundles dmg` 覆盖项目默认 NSIS 目标，没有修改 Windows 打包配置。
- 构建时临时设置 `bundle.macOS.signingIdentity` 为 `-`，使用 macOS 免费 ad-hoc 签名；同时关闭 hardened runtime，不读取或配置开发者证书。
- Tauri 明确跳过 Apple 公证，没有使用 Apple ID、Team ID、API Key 或公证密码。
- 最终生成 `ChuckleChick_0.1.0_aarch64.dmg`，目标为 Apple Silicon。
- 2026-08-12：用户反馈安装后应用图标为空。检查确认黄鸡 PNG 与 `icon.icns` 内容正确，但初版应用包的 `Info.plist` 没有 `CFBundleIconFile`，`Contents/Resources` 也未包含图标；在 Tauri `bundle.icon` 中显式声明 macOS、Windows 所需图标资源后重新打包。
- 修复后应用包的 `Info.plist` 正确声明 `CFBundleIconFile=icon.icns`，`Contents/Resources/icon.icns` 随包发布；没有重新生成或改变黄鸡形象。

## 验证记录

- 2026-08-12：前端类型检查及 Vite 生产构建通过，Rust release 构建通过；仅保留已有的 Tauri API 动静态混合导入非阻断提示。
- 2026-08-12：DMG 通过 `hdiutil` CRC 校验并以只读方式成功挂载，内容包含 `ChuckleChick.app` 和指向 `/Applications` 的快捷入口。
- 2026-08-12：应用主程序经 `file` 与 `lipo` 确认为 Mach-O 64-bit arm64。
- 2026-08-12：`codesign --verify --deep --strict` 通过；签名为 `adhoc`，Identifier 为 `com.chucklechick.desktop`，无 TeamIdentifier。
- 2026-08-12：`spctl` 按预期拒绝未公证应用；用户首次打开时需要通过 Finder 右键“打开”或系统“隐私与安全性”手动允许。
- 2026-08-12：最终 DMG 大小为 `6,268,516` 字节，SHA-256 为 `0e9666ae15471bea82f8f8410b832c862bfb15c762fbd8e0b202863ebcb3b5d1`。
- 产物路径：`src-tauri/target/release/bundle/dmg/ChuckleChick_0.1.0_aarch64.dmg`。
- 2026-08-12：图标修复版重新完成前端生产构建、Rust release 构建和 ad-hoc 签名；DMG 内应用通过 `codesign --verify --deep --strict`。
- 2026-08-12：挂载检查确认 `CFBundleIconFile=icon.icns`，图标文件为有效的 `ic10` macOS 图标，内含 `1024 × 1024 RGBA` 黄鸡图；修复版 DMG 大小为 `8,525,466` 字节，SHA-256 为 `7a5e941a7db3bde54f94f814285516072f2b4e075f596e89b5577245aca16dab`。

## 变更记录

| 日期 | 变更内容 |
| --- | --- |
| 2026-08-12 | 用户确认生成无需证书的本地 macOS DMG，需求状态直接进入开发中 |
| 2026-08-12 | 完成 ad-hoc 签名的 Apple Silicon DMG 构建与挂载校验，状态更新为已完成 |
| 2026-08-12 | 用户反馈安装后图标为空，确认根因为打包配置未显式声明图标；状态恢复为开发中 |
| 2026-08-12 | 显式打包现有黄鸡图标并完成 DMG 回归验证，状态恢复为已完成 |
