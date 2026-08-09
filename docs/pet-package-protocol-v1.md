# MomoPet 宠物包协议 V1

本文档定义 MomoPet 外部宠物作者与 MomoPet 应用之间的稳定文件契约。文中的“必须”“不得”和“应当”是规范性要求。

## 1. 设计目标

- 作者可以独立制作、校验和分发单文件宠物包。
- 同一宠物跨版本保持稳定身份，用户选择和动作快捷键不会因素材更新失效。
- 所有行为均为声明式数据；宠物包不能执行脚本、插件或远程代码。
- 内建宠物与外部宠物使用同一份 manifest 和校验规则；是否内建由安装来源决定，不能由包自行声明。

V1 只支持 `live2d-cubism` 运行时，不支持皮肤、变体、在线依赖或任意脚本。未来运行时通过新的 profile 扩展，不改变核心包结构。

## 2. 容器与目录

宠物包扩展名必须是 `.momopet`，内容是 ZIP。ZIP 根目录必须直接包含 `manifest.json`，不得额外套一层目录。

```text
example.momopet
├── manifest.json
├── LICENSE.txt
├── model/
│   ├── pet.model3.json
│   ├── pet.moc3
│   ├── textures/texture_00.png
│   ├── motions/idle.motion3.json
│   └── expressions/smile.exp3.json
└── resources/
    ├── cover.png
    ├── background.png
    ├── left-keys/
    └── right-keys/
```

所有路径必须使用 `/`，由可移植 ASCII 字符组成，并区分大小写。不得包含绝对路径、`.`、`..`、空段、反斜杠、NUL、符号链接或重复路径。作者应使用英文小写文件名减少跨平台差异。

## 3. Manifest

完整示例见 [`examples/pet-package/manifest.example.json`](../examples/pet-package/manifest.example.json)，机器可读约束见 [`schemas/momopet-package-v1.schema.json`](../schemas/momopet-package-v1.schema.json)。

核心字段：

- `protocolVersion`：必须为整数 `1`。
- `id`：作者控制的稳定反向域名 ID，例如 `com.example.momo`；素材更新不得改变。
- `version`：SemVer 包版本，例如 `1.2.0`。
- `name`、`description`：用户可见信息。
- `authors`：至少一个作者；URL 仅用于展示，不会在导入时访问。
- `license`：必须声明名称并附带包内许可文件；该声明不能替代真实再分发授权。
- `runtime`：V1 固定为 `live2d-cubism` profile `1`，`entry` 指向唯一的 `.model3.json`。
- `presentation.cover`：必需的 PNG 封面；推荐 4:3。`background` 是可选 PNG。
- `actions`：稳定动作目录，必须包含 ID 为 `idle` 的 motion 动作。
- `input`：可选输入可视化映射。缺失时应用不得假定模型实现了任何输入参数。
- `extensions`：可选的命名空间扩展数据。V1 应用保存但不执行扩展内容。

未知核心字段必须被拒绝。因此 `skins`、`variants` 等未进入 V1 的字段不会被静默接受。

## 4. 自定义动作

动作 ID 必须在一个宠物的所有版本中保持稳定。应用持久化和快捷键使用 `<pet-id>:action:<action-id>`，不得使用数组下标作为外部身份。

V1 支持两种动作：

```json
{
  "id": "wave",
  "name": "挥手",
  "type": "motion",
  "motionGroup": "Wave",
  "motionIndex": 0
}
```

```json
{
  "id": "smile",
  "name": "微笑",
  "type": "expression",
  "expression": "smile"
}
```

`motionGroup`/`motionIndex` 必须引用 model3 中存在的 motion，`expression` 必须引用存在的 expression 名称。`idle` 必须是 motion。动作可以由用户快捷键或应用定义的安全事件触发；manifest 不能注册系统快捷键，也不能携带执行逻辑。

## 5. 输入映射

`input` 是可选能力，包含 `standard`、`keyboard` 或 `gamepad` 模式以及 Live2D 参数映射。按键覆盖图必须是包根 `resources/left-keys/` 与 `resources/right-keys/` 的直接子 PNG 文件，文件名去除扩展名后作为输入名称。V1 的 Live2D `Textures` 也只接受 PNG。

参数映射只描述语义到 Live2D Parameter ID 的关系，不允许表达代码。`input.parameters` 至少包含一项非空映射；`gamepad` 模式必须提供至少一项非空的 `gamepad` 映射。

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

只允许 ZIP Stored 与 Deflate 压缩。加密条目、符号链接、设备文件和已知可执行/脚本扩展名必须被拒绝。导入必须先在同一仓库内的临时目录完成全部校验，再激活新目录；升级不得在现有目录中原地解压，并必须保留失败回滚路径。

同一 `id` 与 `version` 的内容必须不可变：内容摘要不同视为冲突。较新 SemVer 可以升级；默认拒绝降级。

## 7. 内建宠物

应用内建宠物使用 `.momopet` 的解包表示，放在 `src-tauri/assets/models/<pet-id>/`。目录名必须与 `manifest.id` 完全一致，内容必须通过与外部包相同的 manifest、引用、PNG 和资源预算校验。ZIP 只是作者分发与用户导入时的容器，不允许再把 `.momopet` 文件嵌套进内建目录。

`isBuiltin` 是应用根据资源来源附加的安装态信息，不是 manifest 字段。外部包不得覆盖同 ID 的内建宠物；内建素材仍必须具备明确的再分发授权。仓库当前不附带 Live2D 宠物素材。

## 8. 兼容策略

- 应用只接受明确支持的协议主版本和 runtime profile。
- V1 核心字段集合冻结；增加、删除或改变核心字段语义，以及放宽执行能力，都必须升级协议主版本。
- 未识别的核心字段拒绝；向后兼容的厂商实验数据只能放入 `extensions`，并使用反向域名键。
- 包声明的 `id`、作者和许可属于元数据，不代表平台已验证发布者身份。

## 9. 作者工作流

```bash
pnpm pet:pack -- path/to/pet-directory output.momopet
pnpm pet:validate -- output.momopet
```

打包命令不会覆盖已有输出文件。校验通过只证明协议和素材引用完整，不证明素材权利、Live2D/Cubism 许可或视觉质量。
