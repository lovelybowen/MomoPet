# MomoPet 宠物包协议 V1

本文档定义 MomoPet 外部宠物作者与应用之间的稳定文件契约。文中的“必须”“不得”和“应当”是规范性要求。

## 1. 设计目标

- 作者只需准备透明 PNG 动画帧和 JSON，不依赖专业建模软件。
- 同一宠物跨版本保持稳定身份，用户选择和动作快捷键不会因素材更新失效。
- 所有行为均为声明式数据；宠物包不能执行脚本、插件或远程代码。
- 内建宠物与外部宠物使用相同的 manifest、Sprite2D 配置和校验规则。

V1 只支持 `sprite2d` runtime profile `1`，由应用内的 PixiJS `AnimatedSprite` 渲染。它可以通过逐帧动画、片段切换、淡入淡出、呼吸微动和指针响应表现动作与表情，但不提供骨骼变形、参数化换装或实时口型。需要这些能力时应增加新的协议主版本，而不是在 V1 中嵌入专有工程文件。

## 2. 容器与目录

宠物包扩展名必须是 `.momopet`，内容是 ZIP。ZIP 根目录必须直接包含 `manifest.json`，不得额外套一层目录。

```text
example.momopet
├── manifest.json
├── LICENSE.txt
├── model/
│   ├── pet.sprite.json
│   └── sprites/
│       ├── idle.png
│       ├── happy.png
│       └── sleep.png
└── resources/
    ├── cover.png
    └── background.png
```

所有路径必须使用 `/`，由可移植 ASCII 字符组成并区分大小写。不得包含绝对路径、`.`、`..`、空段、反斜杠、NUL、符号链接或重复路径。作者应使用英文小写文件名减少跨平台差异。

## 3. Manifest

完整示例见 [`examples/pet-package/manifest.example.json`](../examples/pet-package/manifest.example.json)，机器可读约束见 [`schemas/momopet-package-v1.schema.json`](../schemas/momopet-package-v1.schema.json)。

核心字段：

- `protocolVersion`：必须为整数 `1`。
- `id`：作者控制的稳定反向域名 ID，例如 `com.example.momo`；素材更新不得改变。
- `version`：SemVer 包版本，例如 `1.2.0`。
- `name`、`description`：用户可见信息。
- `authors`：一至八个作者；URL 仅用于展示，导入时不会访问。
- `license`：必须声明名称并附带包内许可文件；该声明不能替代真实再分发授权。
- `runtime`：V1 固定为 `sprite2d` profile `1`，`entry` 指向包中唯一的 `.sprite.json`。
- `presentation.cover`：必需的 PNG 封面；`background` 是可选 PNG。
- `actions`：可选的稳定动作目录。
- `extensions`：可选的命名空间扩展数据；V1 应用保存但不执行其中内容。

未知核心字段必须被拒绝。因此 `input`、`skins`、`variants` 等未进入 V1 的字段不会被静默接受。

## 4. Sprite2D 配置

Sprite2D 配置示例见 [`examples/pet-package/model/pet.sprite.example.json`](../examples/pet-package/model/pet.sprite.example.json)，Schema 见 [`schemas/momopet-sprite-v1.schema.json`](../schemas/momopet-sprite-v1.schema.json)。

```json
{
  "frameSize": { "width": 512, "height": 512 },
  "anchor": { "x": 0.5, "y": 1.0 },
  "sheets": { "main": "sprites/main.png" },
  "clips": {
    "idle": {
      "sheet": "main",
      "frames": [0, 1, 2, 1],
      "fps": 6,
      "loop": true
    }
  }
}
```

- `frameSize`：所有精灵表共享的单帧宽高，单位为像素。
- `anchor`：可选锚点，`x`、`y` 范围均为 `0..1`；默认值为底部中心 `{ "x": 0.5, "y": 1.0 }`。
- `sheets`：一至 32 张带 alpha 通道的 PNG；路径相对于 `.sprite.json` 所在目录。
- `clips`：一至 128 个动画片段。每个片段引用精灵表、帧序号、`1..60` FPS 和循环标志。
- `interactions.tap`：可选的点击动作 ID。目前 V1 只定义 `tap` 交互。

精灵表必须是无间距、无边框的统一网格，按从左到右、从上到下的行优先顺序编号，首帧为 `0`。宽高必须能被 `frameSize` 整除。每个片段最多引用 512 帧；重复帧序号是合法的，可用于控制停顿节奏。

`clips.idle` 必须存在且必须循环。每个被引用帧都必须包含非透明像素，四个角必须完全透明；这样可以及早发现未去除的底色、错误切片和空帧。

## 5. 自定义动作

动作 ID 必须在一个宠物的所有版本中保持稳定。应用持久化和快捷键使用 `<pet-id>:action:<action-id>`，不得使用数组下标作为外部身份。

```json
{
  "id": "happy",
  "name": "开心",
  "type": "animation",
  "clip": "happy",
  "mode": "once"
}
```

`clip` 必须引用 Sprite2D 配置中的片段。`mode` 只有两种：

- `once`：播放一次后自动回到 `idle`。
- `toggle`：再次触发时回到 `idle`，并且目标片段必须设置为循环。

`idle` 属于运行时的必备基础片段，不需要在 `actions` 中暴露。Manifest 不能注册系统快捷键，也不能携带执行逻辑。

## 6. 安全与资源预算

| 项目 | V1 上限 |
| --- | ---: |
| `.momopet` 文件 | 256 MiB |
| ZIP 条目数 | 1024 |
| 解压后总大小 | 512 MiB |
| 单文件大小 | 128 MiB |
| `manifest.json` | 256 KiB |
| 单条路径 | 240 字节 |
| PNG 宽或高 | 8192 px |
| PNG 总像素 | 33,554,432 |

只允许 ZIP Stored 与 Deflate 压缩。加密条目、符号链接、设备文件和已知可执行或脚本扩展名必须被拒绝。导入必须先在同一仓库内的临时目录完成全部校验，再激活新目录；升级不得在现有目录中原地解压，并必须保留失败回滚路径。

同一 `id` 与 `version` 的内容必须不可变：内容摘要不同视为冲突。较新 SemVer 可以升级；默认拒绝降级。

## 7. 内建宠物与存储

应用内建宠物使用 `.momopet` 的解包表示，放在 `src-tauri/assets/models/<pet-id>/`。目录名必须与 `manifest.id` 完全一致，内容必须通过与外部包相同的校验。ZIP 只用于作者分发和用户导入，不得嵌套进内建目录。

`isBuiltin` 是应用按来源附加的安装态信息，不是 manifest 字段。外部包不得覆盖同 ID 的内建宠物。应用附带的 Momo 也必须保留独立的素材来源和再分发说明。

外部包安装到应用数据目录的 `pets/`。由于本协议在应用首次发布前直接替换旧方案，应用不会读取或转换旧模型；启动时会删除当前应用数据目录，以及旧开发标识 `com.4096bytes.momopet.live2d` 目录中精确命名为 `custom-models/` 的子目录，不会删除这两个应用目录中的其他内容，也不会扫描其他路径。

## 8. 兼容策略

- 应用只接受明确支持的协议主版本和 runtime profile。
- V1 核心字段集合冻结；增加、删除或改变核心字段语义必须升级协议主版本。
- 未识别的核心字段拒绝；厂商实验数据只能放入 `extensions`，并使用反向域名键。
- 包声明的 `id`、作者和许可属于元数据，不代表平台已验证发布者身份。
- V1 不兼容旧运行时工程或旧目录结构；这是发布前的一次性协议替换。

## 9. 作者工作流

按文件名排序的透明 PNG 帧可以直接打成统一网格精灵表：

```bash
pnpm pet:spritesheet -- path/to/frames model/sprites/happy.png 4
pnpm pet:validate-dir -- path/to/pet-directory
pnpm pet:pack -- path/to/pet-directory output.momopet
pnpm pet:validate -- output.momopet
```

命令不会覆盖已有输出文件。校验通过只证明协议、素材引用和技术约束完整，不证明素材权利或视觉质量。
