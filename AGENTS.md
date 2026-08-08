# MomoPet GitHub Flow

本仓库采用只有一个长期分支的 GitHub Flow。`master` 始终代表可继续开发、可用于发布准备的主线；不要创建长期存在的 `develop`、`release` 或 `hotfix` 分支。

## 开始工作

从最新远程主线创建一个单一目的的短期分支：

```bash
git switch master
git pull --ff-only origin master
branch_name="chore/update-ci"
git switch -c "$branch_name"
```

将示例变量替换为实际分支名；分支类型使用 `feat`、`fix`、`docs` 或 `chore`。保持分支聚焦，避免在同一个 PR 中混入无关重构。

## 提交与验证

- 提交信息和 PR 标题都必须符合 Conventional Commits，例如 `feat: add model preview`。
- 提交前运行与改动相关的检查；发起 PR 前，在平台依赖可用时运行完整的 `pnpm check`。
- 不得通过降低 lint、类型、测试或构建门禁来绕过失败。

## Pull Request

- 将短期分支推送到远程，并只向 `master` 发起 PR；禁止直接推送 `master`。
- 合并前必须等待 `Frontend`、`Desktop (Windows)`、`Desktop (macOS)` 和 `Desktop (Linux-X11)` 全部成功。
- 仓库不要求人工批准，但零批准不代表可以绕过 CI 或分支保护。
- 只使用 Squash and merge，使 PR 标题成为主线提交标题。

## 合并后清理

GitHub 会自动删除已合并的远程短期分支。Squash 后原分支提交不是 `master` 的祖先，因此普通的 `git branch -d` 可能拒绝删除。先在 GitHub 确认 PR 已合并，再验证远程查询无输出且本地分支与 `master` 内容一致：

```bash
branch_name="chore/update-ci"
git switch master
git pull --ff-only origin master
git fetch --prune origin
git ls-remote --heads origin "refs/heads/$branch_name"
git diff --quiet master "$branch_name"
git branch -D "$branch_name"
```

只有前两项证明都成立时才可执行最后的强制删除；否则保留本地分支并先查明差异。

## 发布

发布标签必须从已同步且通过门禁的 `master` 创建。运行发布检查清单和 `pnpm release:preflight` 后才可推送 `v*` 标签；标签工作流只创建 Draft Release。
