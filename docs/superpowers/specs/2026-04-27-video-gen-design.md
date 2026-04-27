---
title: video-gen — 图片 + 音频合成视频（v0.1）
date: 2026-04-27
status: approved
---

# video-gen 设计文档

## 1. 目标与范围

将一组图片和一组音频合成为单个 mp4 视频。每张图片配一段对应的音频，按文件名顺序串行排布；支持视频整体首尾留白和图片之间的留白。

第一版只做 headless CLI，一个子命令 `build`，零交互。配对策略严格 1:1（同 basename 不同扩展名）。

非目标（v0.1 不做）：

- 在线/REPL UI
- 一图多音、多图一音、组内布局
- 图片间转场动画（淡入淡出、滑动等）
- 字幕、水印、背景音乐
- 视频源输入（只接受静态图片）

## 2. 时间线模型

### 2.1 配对（discover）

输入根目录默认 `input/`，可用 `-d/--input-dir` 覆盖。扫描这三个位置（不递归更深）的并集：

- `<root>/`
- `<root>/images/`
- `<root>/audio/`

按扩展名归类：

- 图片：`.jpg .jpeg .png .webp`
- 音频：`.mp3 .wav .m4a .flac`
- 其他扩展名静默忽略

配对 key 是去掉扩展名的 basename。冲突处理：

| 情况 | 行为 |
|---|---|
| 1 张图 + 1 段音频，同 basename | 合法的 Unit |
| 同 basename 多张图或多段音频 | fail-fast，错误信息提示 "v0.1 only supports 1:1 pairing — 请合并或重命名" |
| 仅图、无音频 | `WARN orphan: <name>.<ext> (no audio)`，跳过 |
| 仅音频、无图 | `WARN orphan: <name>.<ext> (no image)`，跳过 |
| 一对都没有（最终 Unit[] 为空） | fail-fast |

### 2.2 校验（早失败，跑 ffmpeg 之前一次性做完）

- 至少 1 个有效 Unit
- 用 ffprobe 读首张图分辨率作为画布尺寸；逐张比对其它图严格相等（宽×高），不一致 fail-fast，错误信息列出具体哪张不匹配
- 用 ffprobe 读每段音频的时长（同时验证音频可读）；任一失败 fail-fast

### 2.3 排序

最终 `Unit[]` 按 basename 字典序排序。不参考路径，避免 `images/01_*` 与根目录 `02_*` 因路径差异乱序。

### 2.4 Unit

```ts
type Unit = {
  basename: string;        // 配对 key、排序键
  imagePath: string;       // 绝对路径
  audioPath: string;       // 绝对路径
  audioDurationMs: number; // ffprobe 读出
};
```

### 2.5 Plan：Unit[] → Segment[]

`pipeline/plan.ts` 是纯函数：

```ts
type PlanOptions = { leadInMs: number; tailMs: number; gapMs: number };

type Segment =
  | { kind: 'lead-in'; image: string; durationMs: number }
  | { kind: 'unit';    image: string; audio: string; durationMs: number }
  | { kind: 'gap';     image: string; durationMs: number }
  | { kind: 'tail';    image: string; durationMs: number };

function plan(units: Unit[], opts: PlanOptions): Segment[]
```

规则：

1. 若 `leadInMs > 0`：产出 `lead-in` 段，画面 = 首 unit 的图，时长 = leadInMs
2. 对每个 unit 顺序产出 `unit` 段，画面 = 该 unit 的图，时长 = 该 unit 的音频时长
3. 相邻两个 unit 之间，若 `gapMs > 0`：产出 `gap` 段，画面 = 前一个 unit 的图，时长 = gapMs
4. 若 `tailMs > 0`：产出 `tail` 段，画面 = 末 unit 的图，时长 = tailMs

时长 0 的段不产出。`leadInMs / tailMs / gapMs` 默认全为 0。

举例（3 unit + lead-in 2s + gap 0.5s + tail 1s）：

```
lead-in(首图 2s) → unit1 → gap(unit1 图 0.5s) → unit2 → gap(unit2 图 0.5s) → unit3 → tail(unit3 图 1s)
```

## 3. 视频规格与编码

### 3.1 画布

- 分辨率：取首张图的实际宽×高，不缩放、不裁剪、不补黑边。其它图必须严格相等。
- 帧率：默认 30，可通过 `--fps` 调整
- 像素格式：固定 `yuv420p`（兼容性硬需求，不暴露）

### 3.2 编码

- 视频：`libx264`，`--crf` 默认 20，`--preset` 默认 `medium`
- 音频：`aac`，`--audio-bitrate` 默认 `192k`
- 容器：mp4，`-movflags +faststart`（写死，让 moov atom 移到文件头）

### 3.3 音频规整

所有段在 ffmpeg filter graph 内统一到 **48000 Hz / stereo**。静音段用 `aevalsrc=0:s=48000:c=stereo` 直接生成，无需中间文件。

实段统一用：

```
-af "aresample=48000,aformat=channel_layouts=stereo"
```

## 4. 合成策略：分段编码 + concat demuxer

### 4.1 阶段 1：每段独立编码

每个 Segment 产出一个中间 mp4：`.video-gen/build-<runId>/seg_NNN.mp4`，编号按 plan 顺序从 001 开始，零填充至少 3 位。

所有段使用**完全一致的视频/音频编码参数**，concat demuxer 才能无重新编码地拼接。

实段（kind='unit'）的 ffmpeg argv 形如：

```
ffmpeg -y \
  -loop 1 -framerate <FPS> -i <IMAGE> \
  -i <AUDIO> \
  -af "aresample=48000,aformat=channel_layouts=stereo" \
  -c:v libx264 -preset <PRESET> -crf <CRF> -pix_fmt yuv420p \
  -c:a aac -b:a <BITRATE> \
  -t <DURATION_SECONDS> -shortest \
  -movflags +faststart \
  seg_NNN.mp4
```

静音段（kind='lead-in' / 'gap' / 'tail'）：

```
ffmpeg -y \
  -loop 1 -framerate <FPS> -i <IMAGE> \
  -f lavfi -i aevalsrc=0:s=48000:c=stereo \
  -c:v libx264 -preset <PRESET> -crf <CRF> -pix_fmt yuv420p \
  -c:a aac -b:a <BITRATE> \
  -t <DURATION_SECONDS> -shortest \
  -movflags +faststart \
  seg_NNN.mp4
```

`<DURATION_SECONDS>` 由 `Segment.durationMs / 1000` 得到，保留三位小数。

### 4.2 阶段 2：concat demuxer

写 `concat.txt`：

```
file 'seg_001.mp4'
file 'seg_002.mp4'
...
```

执行：

```
ffmpeg -y -f concat -safe 0 -i concat.txt -c copy -movflags +faststart <OUTPUT>
```

只 mux 不重编。

### 4.3 调度

第一版**串行**编码所有段。理由：ffmpeg 单进程通常已能吃满 CPU；串行的进度日志更易读；I/O 不是瓶颈。

如果将来有跨段并行需求再加 `--concurrency`，但 v0.1 不支持。

### 4.4 中间文件管理

目录：`.video-gen/build-<runId>/`，`<runId>` 用启动时间戳（`build-20260427-184201`）。

清理策略：

- 成功：默认整目录删除
- 失败：保留供 debug
- `--keep-temp`：成功也保留

`.gitignore` 必须包含 `.video-gen/`。

## 5. CLI

### 5.1 入口与子命令

```bash
video-gen build [options]
```

`video-gen --help` 列出 `build`；`video-gen build --help` 给详细 flag 列表。第一版只有这一个子命令。

### 5.2 Flags

| flag | 默认 | 作用 |
|---|---|---|
| `-d, --input-dir <dir>` | `input` | 扫这个目录及其 `images/` `audio/` 子目录 |
| `-o, --output <path>` | `output/<input-dir-name>.mp4` | 输出路径 |
| `--lead-in <sec>` | `0` | 开头留白（首图、静音） |
| `--tail <sec>` | `0` | 结尾留白（末图、静音） |
| `--gap <sec>` | `0` | 单元间留白（上一张图、静音） |
| `--fps <n>` | `30` | 帧率 |
| `--crf <n>` | `20` | x264 画质（小=好） |
| `--preset <name>` | `medium` | x264 预设 |
| `--audio-bitrate <bps>` | `192k` | aac 码率 |
| `--keep-temp` | off | 成功也保留 `.video-gen/build-<runId>/` |
| `--quiet` | off | 抑制逐段进度，仅最终结果 |
| `--json` | off | JSON 行式输出 |

时长参数（`--lead-in / --tail / --gap`）接受小数秒。

### 5.3 进度输出（默认 / 人类可读）

```
[discover] 8 units (3 orphans skipped)
[plan]     11 segments (lead-in 2.0s, 8x unit, 2x gap 0.5s, tail 1.0s) total 47.2s
[build]    [ 1/11] seg_001 lead-in              2.000s ... ok (0.4s)
[build]    [ 2/11] seg_002 unit 01_intro        4.217s ... ok (1.1s)
[build]    [ 3/11] seg_003 gap                  0.500s ... ok (0.2s)
...
[concat]   output/myvid.mp4 (12.8MB, 47.2s)
done in 18.4s
```

`--quiet`：仅最终一行 `done: <output> (<size>, <duration>)` 或错误信息。

`--json`：每行一个 JSON event：

```jsonl
{"phase":"discover","units":8,"orphans":3}
{"phase":"plan","segments":11,"total_ms":47200}
{"phase":"build","seg":"seg_001","kind":"lead-in","duration_ms":2000,"status":"ok","ms":400}
{"phase":"build","seg":"seg_002","kind":"unit","basename":"01_intro","duration_ms":4217,"status":"ok","ms":1100}
{"phase":"concat","output":"output/myvid.mp4","bytes":13421772,"duration_ms":47200}
{"phase":"done","ms":18400}
```

### 5.4 退出码

| code | 含义 |
|---|---|
| 0 | 成功 |
| 1 | 运行时错误（ffmpeg 失败、I/O 错误等） |
| 2 | 用户错误（参数错、ffmpeg 找不到、配对冲突、目录为空、图片尺寸不一致） |

错误信息样式（人类模式）：

```
[error] image dimensions mismatch:
  expected 1920x1080 (from 01_intro.jpg)
  got      1280x720  in    03_details.png
```

```
[error] ambiguous basename: 01_intro
  matched 2 images: input/01_intro.jpg, input/images/01_intro.png
  v0.1 only supports 1:1 pairing — 请合并或重命名
```

`--json` 下错误以 JSON 行输出 `{"phase":"error","code":2,"message":"..."}`，再退出。

### 5.5 失败时回显 ffmpeg stderr

任一段 ffmpeg 退出码非 0：fail-fast。把那段的 stderr 末尾约 20 行回显（ffmpeg 的有用错误信息总在尾部），保留 `.video-gen/build-<runId>/` 目录。

## 6. ffmpeg / ffprobe 解析

延续 voice-gen 的策略，二者各自独立解析，三层优先级：

1. 环境变量 `VIDEO_GEN_FFMPEG` / `VIDEO_GEN_FFPROBE`（覆盖路径）
2. `process.execPath` 同目录下的 `ffmpeg` / `ffprobe`（release tarball 附带）
3. fallback 到 `$PATH`

启动时各跑一次 `<bin> -version` 探活，拿不到立即 fail-fast，提示 `brew install ffmpeg` 或下载 release。

Release 构建必须把对应平台的 `ffmpeg` 与 `ffprobe` 静态二进制一并放进 tarball/zip。

## 7. 项目结构

```
package.json        # bun + commander 一个 deps
tsconfig.json       # 沿用 voice-gen 风格（ES2022 / Node16 / strict）
src/
  index.ts                 # 入口，spawn commander
  bin/
    headless.ts            # build 子命令注册
    format.ts              # table / kv 工具（沿用 voice-gen 风格）
  pipeline/
    discover.ts            # 扫目录、配对、孤儿告警 → Unit[]
    plan.ts                # Unit[] + 时长参数 → Segment[]，纯函数
    build.ts               # 调度：probe → encode segments → concat → cleanup
  ffmpeg/
    resolve.ts             # 解析 ffmpeg / ffprobe 路径
    probe.ts               # ffprobe 包装：读音频时长、图片尺寸
    encode.ts              # 构造 ffmpeg argv、spawn、stderr 整理
  store/
    paths.ts               # .video-gen/build-<runId>/ 管理 + 清理
  log.ts                   # 进度输出，text/json/quiet 三模式
tests/
  pipeline/plan.test.ts
  pipeline/discover.test.ts
  ffmpeg/encode.test.ts
  e2e/build.test.ts
input/.gitkeep
output/.gitkeep
.gitignore                 # node_modules/ dist/ output/* input/* .video-gen/
```

## 8. 测试策略

| 层 | 内容 | 是否需要真 ffmpeg |
|---|---|---|
| `plan` 单测 | 边界（时长为 0 不产段）、空 / 单 / 多 unit、各 gap 组合、首尾不产生 gap | 否 |
| `discover` 单测 | fixture 目录覆盖：1:1 成功、孤儿 WARN、多对多 fail、空目录 fail、子目录扫描、扩展名过滤 | 否 |
| `encode` 单测 | argv 组装函数返回数组，断言形状（参数顺序、值正确） | 否 |
| `probe` 单测 | mock spawn，断言时长解析正确 | 否 |
| `build` e2e | 1 张 1×1 png + 1 段 1 秒静音 wav 跑完 build，断言 mp4 存在、ffprobe 读出来时长在 ±0.1s 内 | 是 |

E2E 在没有 ffmpeg 的环境用 `it.skipIf(!hasFfmpeg)` 跳过。CI 装 ffmpeg。

## 9. 依赖与构建

### 9.1 运行时依赖

`commander` 一个，对齐 voice-gen 的 headless 风格。

### 9.2 dev 依赖

`@types/node`、`typescript`、`vitest`，以及 bun 自带的运行时。

### 9.3 npm scripts

```
"dev": "bun src/index.ts"
"build": "bun build src/index.ts --target=bun --outdir dist"
"package": "bun build src/index.ts --compile --outfile dist/video-gen"
"test": "vitest run"
"typecheck": "tsc -p tsconfig.json --noEmit"
```

### 9.4 release tarball 内容

- 平台对应的 `video-gen` 单二进制（bun --compile 产物）
- 平台对应的 `ffmpeg` 静态二进制
- 平台对应的 `ffprobe` 静态二进制
- 短 README

放在同一目录解压即用。

## 10. 已经讨论但 v0.1 排除的能力

记录在此供未来迭代参考，不在当前实现范围：

- **多图多音组内布局**：用 sidecar JSON 描述组内顺序、单段时长。属于"用过几次后真有需要再加"的特性。
- **Web 配置 UI**：浏览器拖拽时间轴、生成 `project.json` 喂给 build。等 sidecar JSON 协议稳定后再做。
- **转场动画**：xfade 等 ffmpeg filter，在 plan 层加 transition Segment 类型。
- **背景音乐**：要在 concat 之后再 mix 一条全局音轨，涉及音量平衡。
- **字幕 / 水印**：subtitles filter / drawtext。
- **段并行编码** `--concurrency`：CPU 多核时可能加速，但实段 ffmpeg 单进程已经多线程，收益不一定大。
- **批量项目** `video-gen batch`：多个目录跑一次。先看实际需求。
