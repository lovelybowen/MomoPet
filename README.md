# MomoPet

MomoPet 是一个基于 Tauri 2、Vue 3、TypeScript、Rust 与 PixiJS 的跨平台 Sprite2D 桌宠。安装后自带 Momo，也支持用户导入只由透明 PNG 精灵表和 JSON 组成的 `.momopet` 宠物包。

## 当前能力

- Windows、macOS 与 Linux X11 的透明主窗口、设置窗口和系统托盘。
- PixiJS 逐帧动画，支持待机循环、一次性动作、可切换状态、片段淡入淡出、呼吸微动和点击交互。
- 受控的 `.momopet` 导入：协议与引用检查、资源预算、路径穿越与符号链接拒绝、稳定 ID、SemVer 升级、内容冲突检测、暂存和回滚替换。
- 宠物包声明的稳定自定义动作及用户快捷键。
- 将按文件名排序的透明 PNG 帧打包为统一网格精灵表的作者 CLI。
- 简体中文、繁体中文、英语、越南语和巴西葡萄牙语界面。

首版不承诺 Wayland。V1 不包含骨骼模型、任意脚本、在线依赖、皮肤或变体。

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

本仓库采用只有一个长期 `master` 分支的 GitHub Flow。所有改动从最新 `master` 创建短期分支，通过必需 CI 后以 Squash 方式合并；不得直接推送 `master`。规则见 [AGENTS.md](AGENTS.md)。

## 宠物包协议

外部作者使用单文件 `.momopet` ZIP 包分发宠物。包根包含 `manifest.json`、许可文件、`.sprite.json` 配置、透明 PNG 精灵表和封面。作者不需要专用建模编辑器。

完整规范见 [宠物包协议 V1](docs/pet-package-protocol-v1.md)，机器可读约束见 [Manifest Schema](schemas/momopet-package-v1.schema.json) 与 [Sprite2D Schema](schemas/momopet-sprite-v1.schema.json)，作者模板见 [示例目录](examples/pet-package)。

```bash
pnpm pet:spritesheet -- path/to/frames output.png 4
pnpm pet:validate-dir -- path/to/pet-directory
pnpm pet:pack -- path/to/pet-directory output.momopet
pnpm pet:validate -- output.momopet
```

内建宠物以同一协议的解包形式放在 `src-tauri/assets/models/<pet-id>/`，并会成为首次启动的默认选择。外部宠物安装到应用数据目录的 `pets/`。

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

完成内建素材再分发确认和三平台真机验证前，不得公开发布 Release。

## 许可与来源

工程代码遵循 [MIT License](LICENSE)。内建宠物素材的生成与再分发记录见其包内 `LICENSE.txt` 和 `ASSET-SOURCES.md`。
