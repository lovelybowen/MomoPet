# MomoPet

MomoPet 是一个基于 Tauri 2、Vue 3、TypeScript、Rust、Pixi.js 与 easy-live2d 的跨平台 Live2D 桌宠工程基线。当前阶段只提供可复用桌宠底座，不包含可再分发的 Live2D 模型，也不包含 Live2D Cubism Core。

## 当前能力

- Windows、macOS 与 Linux X11 的透明主窗口、设置窗口和系统托盘。
- Live2D 模型加载、切换、动作、表情、音效、FPS、缩放与窗口交互链路。
- 受控的 `.momopet` 宠物包导入：协议与引用完整性检查、资源预算、路径穿越与符号链接拒绝、稳定 ID、SemVer 升级、内容冲突检测、同目录暂存与回滚替换，以及受限删除。
- 由宠物包声明的稳定自定义动作目录，支持 Live2D motion 与 expression，并可为每个动作配置快捷键。
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

## 宠物包协议

外部作者使用单文件 `.momopet` ZIP 包分发宠物。包根必须包含 `manifest.json`，声明稳定的反向域名 ID、SemVer 版本、作者、许可、Live2D 入口、封面与动作目录。入口引用的 Moc、纹理、动作、表情、音频和其他运行时文件必须存在，且所有路径都不能使用绝对路径、`..` 或符号链接。

V1 支持作者自定义 motion 与 expression 动作，以及可选的键盘、鼠标和手柄参数映射；暂不支持皮肤、变体、脚本和在线依赖。完整规范见 [宠物包协议 V1](docs/pet-package-protocol-v1.md)，机器可读约束见 [JSON Schema](schemas/momopet-package-v1.schema.json)，作者模板见 [manifest 示例](examples/pet-package/manifest.example.json)。

```bash
pnpm pet:pack -- path/to/pet-directory output.momopet
pnpm pet:validate -- output.momopet
```

已有的旧版目录导入结果仍可读取和删除，但界面只接受新协议包。内建宠物以同一协议的解包形式放在 `src-tauri/assets/models/<pet-id>/`，会优先成为首次启动的默认选择；该目录当前为空，等待具有明确再分发授权的素材。

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
