---
title: video-gen release pipeline (multi-platform tarball + ffmpeg/ffprobe bundle)
date: 2026-04-27
status: approved
---

# video-gen release pipeline 设计

## 1. 目标与范围

每次推送 `v*` tag（或手动触发）时，在 GitHub Actions 上为四个平台分别产出一个解压即用的 archive：内含 `video-gen` 单二进制、`ffmpeg`、`ffprobe`、`README.md`、`LICENSE`。最终上传到 GitHub Release，附 SHA256 校验文件，自动生成 release notes。

非目标（不做）：

- macOS code signing / Apple notarization
- Windows code signing
- 自动 changelog 编辑、版本号自动 bump
- 发布到 npm / Homebrew / Scoop
- darwin-x64（Apple Silicon 时代）
- Docker image
- 增量发布（dev / beta / rc 通道）

## 2. 平台矩阵

| target | runner | archive | binary suffix |
|---|---|---|---|
| darwin-arm64 | `macos-14` | `tar.gz` | (none) |
| linux-x64 | `ubuntu-22.04` | `tar.gz` | (none) |
| linux-arm64 | `ubuntu-24.04-arm` | `tar.gz` | (none) |
| windows-x64 | `windows-latest` | `zip` | `.exe` |

跟 voice-gen `release.yml` 完全一致。`darwin-x64` 不做（用户决策，2026 年 Apple Silicon 已经普及）。

## 3. 触发器

```yaml
on:
  push:
    tags: ['v*']
  workflow_dispatch:
    inputs:
      tag:
        description: 'tag name to publish under (e.g. v0.1.0). leave empty for dry-run (artifacts only).'
        required: false
        default: ''
```

三种实际入口：

| 入口 | tag 来源 | 行为 |
|---|---|---|
| `git push --tags` 推送 `v*` | `${{ github.ref_name }}` | build + 发 release |
| Actions 页面手动触发，填了 tag | `${{ github.event.inputs.tag }}` | build + 用该 tag 名发 release |
| Actions 页面手动触发，tag 留空 | — | 仅 build artifacts，不发 release（dry-run） |

## 4. ffmpeg / ffprobe 来源

用 npm 包 `ffmpeg-static` + `ffprobe-static`：

- `ffmpeg-static@^5.2.0`：跟 voice-gen 一致
- `ffprobe-static@^3.1.0`：与 ffmpeg-static 互补

两个包都按 `process.platform` × `process.arch` 提供对应的预编译静态二进制。在每个 matrix runner 上 `bun add` 后用一段一次性 inline 脚本 resolve 路径并复制到 `dist/`。

为什么不用 `BtbN/FFmpeg-Builds`：voice-gen 已验证 npm 路径稳定可行；BtbN 多一层 download/unzip/checksum，复杂度↑而收益有限（v0.1 的目标平台它们都覆盖）。

## 5. Tarball / zip 内容

每个 archive 解压后得到一个目录 `video-gen-<target>/`，内含：

```
video-gen-<target>/
  video-gen[.exe]      # 主二进制
  ffmpeg[.exe]         # 静态 ffmpeg
  ffprobe[.exe]        # 静态 ffprobe
  README.md            # 仓库根的 README
  LICENSE              # MIT 许可证
```

archive 命名（无版本号）：

- `video-gen-darwin-arm64.tar.gz` + `.sha256`
- `video-gen-linux-x64.tar.gz` + `.sha256`
- `video-gen-linux-arm64.tar.gz` + `.sha256`
- `video-gen-windows-x64.zip` + `.sha256`

SHA256 文件内容遵循 `shasum -a 256` / `Get-FileHash` 单行格式：`<hash>  <archive-name>`。

## 6. LICENSE

仓库根新增 `LICENSE` 文件，MIT 协议，版权人 `nomagicln`。release 实施前的前置任务（plan 第一步），落仓后被 release workflow 复制进每个 tarball。

## 7. README 内容补充

实施时给 `README.md` 加一节"下载 release 后第一次运行"，主要是 macOS Gatekeeper 的应对：

```bash
# macOS：第一次运行被 Gatekeeper 拦截时
xattr -d com.apple.quarantine ./video-gen ./ffmpeg ./ffprobe
```

简短说明"二进制未做 Apple notarization，自行解除 quarantine"。Linux / Windows 不需要类似步骤。

## 8. CI 测试策略

每个 matrix runner 上跑完整 `bun run test`（含 e2e）：

- 在 typecheck + test 之前先 `bun add ffmpeg-static ffprobe-static`，并把 `ffmpeg` / `ffprobe` 复制到 `dist/`
- `tests/e2e/fixtures.ts` 已经走 `resolveBinary`；通过环境变量 `VIDEO_GEN_FFMPEG` / `VIDEO_GEN_FFPROBE` 指向 `dist/{ffmpeg,ffprobe}` 让 e2e 用 bundled 二进制
- 不依赖 runner 自带的 ffmpeg 版本，且发版前再次确认 bundle 的二进制 + e2e 协同正确

收益：发版前最后一次跑通"用户拿到的 ffmpeg + 用户拿到的 video-gen"组合，避免上游 `ffmpeg-static` 升级偷偷改配置导致 e2e 在用户手里失败。代价：每个 runner 多 ~5 秒。

## 9. Workflow 步骤详解

### Job `build`（matrix）

按顺序：

1. **`actions/checkout@v4`**
2. **`oven-sh/setup-bun@v2`** with `bun-version: latest`
3. **install deps**：`bun install --frozen-lockfile`
4. **bundle ffmpeg + ffprobe**：
   ```bash
   bun add ffmpeg-static@^5.2.0 ffprobe-static@^3.1.0
   bun -e "<inline script>"  # resolve 两个 path、复制到 dist/、chmod +x（非 Windows）
   ```
   inline 脚本同时校验两个包都解析到了二进制，否则 fail。
5. **typecheck**：`bun run typecheck`
6. **test**：`VIDEO_GEN_FFMPEG=$PWD/dist/ffmpeg VIDEO_GEN_FFPROBE=$PWD/dist/ffprobe bun run test`（Windows 用 pwsh 等价语法）
7. **compile binary**：`bun build src/index.ts --compile --minify --outfile dist/video-gen${{ matrix.ext }}`
8. **smoke test**：`./dist/video-gen --help | head -20` + `./dist/ffmpeg -version | head -1` + `./dist/ffprobe -version | head -1`（Windows 用 pwsh）
9. **stage release tree**：`mkdir -p release/<name>` + 复制 5 个文件
10. **archive**：tar.gz（macOS / Linux）或 zip（Windows）+ 写 SHA256
11. **`actions/upload-artifact@v4`**：name = `video-gen-<target>`，上传 archive + sha256

### Job `release`

- `needs: build`
- 守卫：`if: startsWith(github.ref, 'refs/tags/v') || (github.event_name == 'workflow_dispatch' && github.event.inputs.tag != '')`
- 步骤：
  1. `actions/download-artifact@v4` with `merge-multiple: true` → 把所有 artifacts 平铺到 `assets/`
  2. `softprops/action-gh-release@v2`：
     ```yaml
     tag_name: ${{ github.event.inputs.tag || github.ref_name }}
     files: assets/*
     generate_release_notes: true
     draft: false
     prerelease: false
     ```

`permissions: contents: write` 在 workflow 级别授予。

## 10. 项目内的相关变更

跟 release pipeline 配套的仓库改动：

- 新增 `.github/workflows/release.yml`
- 新增 `LICENSE`（MIT）
- 修改 `README.md`：加 macOS quarantine 说明小节、加"从 release 下载"使用提示
- 修改 `.gitignore`：加 `dist/`（v0.1 没加，本地 build 后会出现 untracked 文件）

不动的：`package.json` 不需要把 `ffmpeg-static` / `ffprobe-static` 写成 dependency（只在 CI 上 `bun add`，不进 lockfile）。

## 11. 测试与验证策略

实施完成后的验证步骤（plan 末尾的 manual smoke）：

1. **dry-run**：在 GitHub Actions 页面手动触发 workflow，tag 字段留空。预期：4 个 artifacts 生成，无 release 创建。
2. **dispatch + 假 tag**：手动触发，tag 字段填 `v0.0.0-dryrun`。预期：build 通过，但**不**真的发 release（先用 `draft: true` 跑一遍，确认无误后改回 `false`，或用一个 throwaway tag 后立刻删除）。
3. **真 tag**：在本地 `git tag v0.1.0 && git push origin v0.1.0`。预期：4 个 artifacts 上传到 `v0.1.0` release，自动生成 release notes。
4. **下载验证**：从 release 下载每个平台的 archive（至少在本机能验的 darwin-arm64），解压后运行 `./video-gen --help`、`./video-gen build -d ... -o ...` 跑一个最小 fixture 验证二进制完整。

无需自动化测试。release pipeline 本身不写单测——它的"测试"是真的发一次版。

## 12. 故障与回滚

- **build 失败**：fail-fast 停在该平台，其他平台继续（`fail-fast: false`）。release job 不会跑（`needs: build` 默认要求所有 needs 成功）。手动决定：是合并 artifacts 部分发版（罕见），还是修后重打 tag。重打 tag 命令：`git tag -d v0.1.0 && git push origin :refs/tags/v0.1.0 && git tag v0.1.0 && git push origin v0.1.0`。
- **release upload 失败**：artifacts 已经在，重新触发 release job（重跑 workflow）通常可恢复。
- **release 已发但内容错了**：在 GitHub UI 上删 release（保留或删 tag），修后重新发版。`softprops/action-gh-release@v2` 默认更新而非 fail，可以重跑同 tag 的 dispatch。

## 13. 已讨论但 v0.1 排除

- **macOS code signing / notarization**：要 Apple Developer 账号 + secrets + 改 workflow，工作量翻倍。未做的代价是用户首次运行被 Gatekeeper 拦，README 给解决方法即可。
- **Windows code signing**：同上，不做。
- **darwin-x64**：Apple Silicon 时代不再支持 Intel Mac。要支持只需 matrix 加一行（`macos-13` runner）。
- **changelog 自动化（git-cliff / conventional-changelog）**：让 `generate_release_notes: true` 先撑着，commit message 已经在按 conventional commits 写。
- **npm / Homebrew / Scoop 分发**：等用户基数成形再说。
- **dev / beta / rc 通道**：暂时不需要。需要时给 tag 加后缀（`v0.2.0-rc.1`）+ `prerelease: true` 即可。
- **静态二进制的 SBOM / 供应链证明**：v0.1 没必要。
