# desktop-pet

一个基于 **Tauri 2 + Vue 3 + TypeScript + Rust** 的桌面宠物应用。  
应用以透明置顶窗口运行，包含宠物状态动画、聊天/语音能力与系统工具调用能力。

## 🐶功能概览

- 桌面宠物主界面（透明、无边框、置顶）
- 宠物成长与状态管理（喂食、睡眠、心情等）
- 聊天与语音助手相关界面与能力
- Rust 侧工具模块（系统状态、时间、截图、命令执行、应用打开、宠物控制等）
- 全局快捷键能力（Tauri 插件）

🐶---->[演示视频](https://www.bilibili.com/video/BV1fG9QBCE5w/?vd_source=7467b6155503c40aa7c64ce91f85c810)

## 🐱技术栈

- 前端：Vue 3、TypeScript、Vite
- 桌面容器：Tauri 2
- 后端：Rust（Tokio、Reqwest、Sysinfo 等）
- 音频处理：cpal、hound、rubato

## 🐭项目结构

```text
.
├── src/                        # Vue 前端
│   ├── App.vue
│   ├── ChatApp.vue
│   ├── SettingsApp.vue
│   ├── StatusApp.vue
│   ├── OnboardingApp.vue
│   ├── composables/            # 业务状态与能力封装（useXxx）
│   ├── assets/pets/            # 宠物 GIF 资源
│   └── styles/
├── src-tauri/                  # Tauri + Rust 后端
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs
│   │   ├── llm/tools/          # 系统工具模块
│   │   ├── audio/
│   │   └── asr/
│   ├── capabilities/
│   ├── gen/schemas/
│   └── tauri.conf.json
├── doc/                        # 设计与需求文档（中文）
├── package.json
└── AGENTS.md                   # 仓库协作约定
```

## 🐰环境要求

- Node.js 18+（建议 LTS）
- npm 9+
- Rust 1.77+（见 `src-tauri/Cargo.toml`）
- 平台对应的 Tauri 依赖环境
  - macOS：Xcode Command Line Tools
  - Windows：MSVC Build Tools

> 首次拉取后请先安装依赖：

```bash
npm install
```

## 🐻开发指南

### 1) 前端开发（仅 Vue）

```bash
npm run dev
```

默认启动 Vite 开发服务器。

### 2) 桌面应用联调（前端 + Rust）

```bash
npm run tauri dev
```

会自动先启动前端开发服务，再拉起 Tauri 桌面应用。

### 3) 前端构建

```bash
npm run build
```

输出目录为 `dist/`。

### 4) 桌面应用打包

```bash
npm run tauri build
```

生成平台对应的可分发安装包/二进制。

### 5) Rust 快速编译检查

```bash
cargo check --manifest-path src-tauri/Cargo.toml
```

用于快速校验后端改动是否可编译。

## 🐼代码规范

- TypeScript / Vue：2 空格缩进，变量与函数使用 `camelCase`，组件使用 `PascalCase`
- Rust：遵循 `rustfmt` 默认格式，函数/模块使用 `snake_case`，结构体/枚举使用 `PascalCase`
- 组合式函数命名统一 `useXxx`（示例：`usePetState.ts`）
- UI 状态逻辑优先放 `src/composables`，系统工具逻辑放 `src-tauri/src/llm/tools`

## 🐨验证与测试

当前仓库未集成完整自动化测试，建议按改动类型执行最小验证：

- 前端改动：`npm run build`
- 桌面联调：`npm run tauri dev` 进行手工 smoke test
- Rust 改动：`cargo check --manifest-path src-tauri/Cargo.toml`

## 🐯常见问题

### 1. `npm run tauri dev` 无法启动

请先确认：

- 已执行 `npm install`
- 本机已安装 Rust 工具链与平台编译依赖
- 端口 `1420` 未被占用（Tauri devUrl 默认使用该端口）

### 2. 修改前端后桌面窗口没有更新

优先检查 Vite 服务是否正常运行；必要时重启 `npm run tauri dev`。

## 🦁提交规范

建议使用 Conventional Commits：

- `feat: ...`
- `fix: ...`
- `refactor: ...`
- `docs: ...`
- `chore: ...`

示例：`fix(llm-tools): handle timeout in command tool`
