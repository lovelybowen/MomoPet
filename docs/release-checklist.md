# 发布检查清单

## 自动门禁

- [ ] `pnpm install --frozen-lockfile`
- [ ] `pnpm check`
- [ ] `pnpm release:preflight`
- [ ] 仓库中不存在 Cubism Core、未授权模型或来源不明品牌素材
- [ ] updater 仍为 disabled，除非 endpoint、公钥和发布签名条件全部完成
- [ ] Draft Release 明确标记 Windows/Linux 未签名及 macOS ad-hoc 签名状态

## 模型与许可

- [ ] Live2D Cubism Core 来自官方 SDK
- [ ] 已完成适用于本产品形态的 Live2D 发布许可审查
- [ ] 内置模型及其纹理、动作、表情、音频、封面和背景均有可再分发记录
- [ ] 打包后的模型可加载，动作、表情和声音正常

## Windows 真机

- [ ] 透明窗口、拖动、穿透、置顶、任务栏与托盘
- [ ] 开机自启动和全局输入权限
- [ ] 100%/125%/150% DPI、多显示器与边界保持

## macOS 真机

- [ ] 透明窗口、NSPanel 行为、拖动、穿透、置顶与托盘
- [ ] 开机自启动和 Input Monitoring 权限引导
- [ ] Retina、多显示器、全屏空间与窗口恢复

## Linux X11 真机

- [ ] 透明窗口、拖动、穿透、置顶、任务栏与托盘
- [ ] 开机自启动与全局输入
- [ ] 多显示器和缩放

Wayland 不属于首版验收范围。自动 CI 构建不能替代上述真机检查。
