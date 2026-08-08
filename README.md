# MomoPet

MomoPet 是一个基于 Tauri 2、Vue 3、TypeScript、Rust、Pixi.js 与 easy-live2d 的跨平台 Live2D 桌宠工程基线。当前阶段只提供可复用桌宠底座，不包含可再分发的 Live2D 模型，也不包含 Live2D Cubism Core。

## 当前能力

- Windows、macOS 与 Linux X11 的透明主窗口、设置窗口和系统托盘。
- Live2D 模型加载、切换、动作、表情、音效、FPS、缩放与窗口交互链路。
- 受控的本地模型导入：唯一入口检查、引用完整性检查、路径穿越与符号链接拒绝、内容哈希去重、原子安装和受限删除。
- 可整体关闭或替换的 `features/input-visualizer` 示例模块，用键盘、鼠标和手柄映射 Live2D 参数。
- 简体中文、繁体中文、英语、越南语和巴西葡萄牙语界面。

首版不承诺 Wayland。仓库不继承任何旧 MomoPet 项目的代码、数据格式或产品约束。

## 开发环境

- Node.js 22
- pnpm 9.12.3
- Rust stable（含 `rustfmt` 与 `clippy`）
- 各平台的 [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)

```bash
corepack enable
pnpm install --frozen-lockfile
pnpm check
pnpm tauri dev
```

## 开发流程

本仓库采用只有一个长期 `master` 分支的 GitHub Flow。所有改动从最新 `master` 创建短期分支，通过必需 CI 后以 Squash 方式合并；不得直接推送 `master`。分支、提交、PR 和合并后清理规则见 [AGENTS.md](AGENTS.md)。

## Live2D 本地依赖

从 Live2D 官方 Cubism SDK for Web 中取得 `live2dcubismcore.min.js`，放到：

```text
public/vendor/live2d/live2dcubismcore.min.js
```

该文件被 `.gitignore` 排除。可运行 `pnpm check:cubism` 检查本地安装；发布前还必须完成许可审查并运行：

```bash
MOMOPET_LIVE2D_LICENSE_ACKNOWLEDGED=1 pnpm release:preflight
```

Cubism Core 受 Live2D 专有协议约束，项目的 MIT License 不覆盖 Cubism Core 或模型素材。可扩展应用在发布前可能需要单独审批与出版许可。详见 [Cubism Core 说明](https://docs.live2d.com/en/cubism-sdk-manual/cubism-core/) 与 [SDK 发布许可](https://www.live2d.com/en/sdk/license/)。

## 模型目录约定

导入目录必须包含且只包含一个 `.model3.json` 入口。入口引用的 Moc、纹理、动作、表情、音频和其他运行时文件必须存在，且引用不能使用绝对路径、`..` 或符号链接。

可选的 `resources/cover.png`、`resources/background.png`、`resources/left-keys/` 与 `resources/right-keys/` 用于模型封面、背景和输入示例覆盖层。内置模型目录当前为空，等待具有明确再分发授权的素材。

## 质量门禁

```bash
pnpm lint
pnpm typecheck
pnpm test
pnpm build
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

CI 在 Windows、macOS 和 Ubuntu X11 上执行 Rust 检查与 `tauri build --no-bundle`。真机验收项目见 [发布检查清单](docs/release-checklist.md)。

## 发布边界

`v*` tag 只创建 Draft GitHub Release。当前未配置 updater endpoint、公钥或签名密钥，因此不生成 updater artifacts，也不显示自动更新设置。未签名 Windows/Linux 产物及仅 ad-hoc 签名的 macOS 产物只能作为诊断构建，不能表述为正式可信发行。

完成 Live2D 授权审查、模型再分发确认和三平台真机验证前，不得公开发布 Release。

## 许可与来源

工程代码遵循 [MIT License](LICENSE)。
