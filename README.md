# ChuckleChick

一个以“黄鸡大笑”为主角、支持持续扩展的 Windows 桌面宠物与助手项目。

## 项目组成

本项目长期维护两部分内容：

1. **需求文档**：统一存放在 [`requirements/`](requirements/)；记录需求背景、范围、验收标准和实施状态。
2. **主体代码与资源**：用于实现已经确认的需求；当前已建立桌面应用基础工作区和首个角色资源。

项目采用“需求文档先行”原则：**每个需求必须先创建需求文档，明确范围与验收标准后，才能进入开发。** 没有对应需求文档的功能，不应直接修改主体代码。

## 工作流程

1. 在 `requirements/` 中复制需求模板并创建需求文档。
2. 完成需求讨论，明确目标、范围、方案约束和验收标准。
3. 将需求状态更新为“已确认”。
4. 基于已确认的需求文档进行开发和验证。
5. 开发完成并验收后，将状态更新为“已完成”，补充实现与验证记录。

详细约定见 [`requirements/README.md`](requirements/README.md)。

## 技术栈

- Tauri 2：桌面生命周期、窗口和后续系统托盘能力。
- Vue 3、TypeScript、Pinia：应用装配、设置界面和状态管理。
- PixiJS 8：PNG 图集裁剪与 Canvas 动画渲染。
- Rust stable：平台能力与受控的 Tauri 后端接口。
- Vite、Vitest、pnpm：开发构建、核心逻辑测试和依赖管理。

明确不接入全局键盘监听、手柄监听、自动更新和 Live2D。完整技术边界见 [`REQ-0002`](requirements/REQ-0002-desktop-pet-technical-foundation.md)。

## 目录结构

```text
assets/pets/       宠物资源包
requirements/      需求、范围、验收与实施记录
src/components/    Vue 界面装配
src/pet-core/      不依赖 Vue、PixiJS 或 Tauri 的核心逻辑
src/pet-assets/    v2 宠物资源解析与校验
src/pet-renderer/  PixiJS 渲染适配
src/platform/      Tauri API 的 TypeScript 边界
src-tauri/         Rust 桌面外壳和最小权限配置
```

## 开发环境

需要 Node.js 20.19 或更高版本、pnpm 11、Rust stable，以及 Tauri 对应平台的系统依赖。当前仓库使用 Node.js 24 作为推荐开发版本。

```bash
pnpm install
pnpm test
pnpm build
pnpm tauri dev
```

只调试前端资源和动画时运行 `pnpm dev`，再访问终端显示的本地地址。完整桌面窗口需要 Rust 工具链并通过 `pnpm tauri dev` 启动。

## 当前内容

仓库目前包含已经验收通过的 **黄鸡大笑** 角色资源：

- `assets/pets/huangji-daxiao/spritesheet.png`：透明背景的 8 × 11 动画图集。
- `assets/pets/huangji-daxiao/pet.json`：宠物身份信息和资源入口配置。
- `assets/pets/huangji-daxiao/atlas.json`：单元格尺寸、动画行、播放时长和观察方向数据。

基础工作区会读取该资源包、校验 v2 元数据并播放 `idle` 动画；透明无边框、固定尺寸、默认置顶和隐藏任务栏由 Tauri 窗口配置声明。

当前首版能力包括：

- 待机期间随机播放挥手、跳跃和评审动作，随后自动回到待机。
- 拖动时按左右方向播放跑动动画；鼠标在宠物周围时使用 16 方向观察帧看向鼠标。
- 托盘显示/隐藏宠物、切换置顶、打开设置和退出应用。
- 最小设置窗口可调整置顶状态与 40%–200% 显示比例；100% 对应约 `144 × 156` 逻辑像素。
- 在应用数据目录保存窗口位置、缩放、置顶状态和当前宠物，并在启动时恢复到有效显示器范围内。

鼠标观察使用 Tauri提供的当前位置查询，窗口隐藏时停止查询；项目不安装全局鼠标事件钩子，也不保存鼠标轨迹。

## 动画图集

- 画布尺寸：`1536 × 2288`
- 图集网格：`8 列 × 11 行`
- 单元格尺寸：`192 × 208`
- 文件格式：透明 PNG
- 第 0–8 行：标准动画状态
- 第 9–10 行：顺时针排列的 16 个观察方向
