---
title: video-gen Rust library + headless CLI rewrite
date: 2026-04-30
status: approved
---

# video-gen Rust 重构设计

## 1. 目标与范围

用 Rust 重写当前 TypeScript/Bun 实现，只保留现有 headless CLI 能力，同时让项目可以作为 Rust crate 被 Tauri 后端直接引用。

保留的用户能力：

- `video-gen build [options]` 命令格式
- 图片 + 音频按 basename 严格 1:1 配对
- 扫描 `<input>/`、`<input>/images/`、`<input>/audio/`
- `lead-in`、`gap`、`tail` 静音段
- ffprobe 读取图片尺寸和音频时长
- ffmpeg 分段编码后用 concat demuxer 拼接
- text / quiet / json 日志模式
- 用户错误退出码 2，运行时错误退出码 1
- release archive 内仍包含 `video-gen`、`ffmpeg`、`ffprobe`、`README.md`、`LICENSE`

非目标：

- GUI、交互式模式、Web API
- 新的视频编辑功能
- Tauri 插件封装
- wasm 或移动端原生编码
- code signing / notarization

## 2. 项目结构

采用单个 Cargo package：`video-gen`。同一个 package 同时提供库和二进制。

```text
Cargo.toml
src/
  lib.rs
  main.rs
  error.rs
  log.rs
  discover.rs
  plan.rs
  ffmpeg.rs
  build.rs
tests/
  discover.rs
  plan.rs
  ffmpeg_args.rs
  ffprobe_parse.rs
  output_path.rs
  e2e.rs
```

职责：

- `lib.rs`：公开 Tauri 可调用 API 与核心类型。
- `main.rs`：CLI 薄封装，解析参数、选择日志输出、调用 `build_video`。
- `error.rs`：统一错误类型和退出码。
- `log.rs`：事件类型、日志模式、CLI 输出适配。
- `discover.rs`：文件发现、配对、orphan、冲突检测。
- `plan.rs`：纯函数 `Unit[] -> Segment[]`。
- `ffmpeg.rs`：二进制解析、ffprobe JSON 解析、argv 构建、进程执行。
- `build.rs`：完整 build pipeline 编排。

删除 TypeScript/Bun 结构：`package.json`、`bun.lock`、`tsconfig.json`、`vitest.config.ts`、`src/*.ts`、`tests/*.ts`。

## 3. 库 API

核心 API：

```rust
pub fn build_video<F>(options: BuildOptions, on_event: F) -> Result<BuildResult, VideoGenError>
where
    F: FnMut(BuildEvent);
```

公开类型：

```rust
pub struct BuildOptions {
    pub input_dir: PathBuf,
    pub output_path: PathBuf,
    pub work_dir: PathBuf,
    pub keep_temp: bool,
    pub plan: PlanOptions,
    pub encode: EncodeOptions,
    pub binaries: BinaryOptions,
}

pub struct PlanOptions {
    pub lead_in_ms: u64,
    pub tail_ms: u64,
    pub gap_ms: u64,
}

pub struct EncodeOptions {
    pub fps: u32,
    pub crf: u8,
    pub preset: String,
    pub audio_bitrate: String,
}

pub struct BinaryOptions {
    pub ffmpeg: Option<PathBuf>,
    pub ffprobe: Option<PathBuf>,
}

pub struct BuildResult {
    pub output: PathBuf,
    pub bytes: u64,
    pub duration_ms: u64,
}
```

Tauri 使用方式：

```rust
video_gen::build_video(options, |event| {
    let _ = app.emit("video-gen-event", event);
})?;
```

`BuildOptions` 和 `BuildEvent` 派生 `Serialize` / `Deserialize`，方便 Tauri command 参数和进度事件序列化。`VideoGenError` 保留错误分类，并提供 `exit_code()`、`message()`，CLI 用它转成退出码。

## 4. CLI

保留原命令：

```bash
video-gen build -d input -o output/myvid.mp4 --lead-in 2 --tail 2 --gap 0.5
```

参数兼容：

| flag | 默认 | 行为 |
|---|---:|---|
| `-d, --input-dir <dir>` | `input` | 扫描输入目录 |
| `-o, --output <path>` | `output/<input-dir-name>.mp4` | 输出路径 |
| `--lead-in <sec>` | `0` | 首图静音段 |
| `--tail <sec>` | `0` | 末图静音段 |
| `--gap <sec>` | `0` | 单元间静音段 |
| `--fps <n>` | `30` | 帧率，范围 1..240 |
| `--crf <n>` | `20` | x264 CRF，范围 0..51 |
| `--preset <name>` | `medium` | x264 preset |
| `--audio-bitrate <bps>` | `192k` | AAC bitrate |
| `--keep-temp` | off | 成功也保留 `.video-gen/<runId>/` |
| `--quiet` | off | 只输出最终结果 |
| `--json` | off | JSONL 事件输出 |

`--output` 行为保持现状：

- 省略时用 `output/<input-dir-name>.mp4`
- 指向已存在目录时追加 `output.mp4`
- 带尾部路径分隔符时追加 `output.mp4`
- 不存在且无扩展名时当目录处理并追加 `output.mp4`
- 不存在但有扩展名时当文件路径处理

## 5. ffmpeg / ffprobe 解析

解析优先级：

1. `BuildOptions.binaries.ffmpeg` / `ffprobe`
2. 环境变量 `VIDEO_GEN_FFMPEG` / `VIDEO_GEN_FFPROBE`
3. 当前可执行文件旁边的 `ffmpeg[.exe]` / `ffprobe[.exe]`
4. `$PATH` 中的 `ffmpeg[.exe]` / `ffprobe[.exe]`

这个顺序让 Tauri 可以显式传 sidecar 路径，也保留 CLI release tarball 的 sibling binary 体验。

## 6. 数据流

`build_video` 的流程：

1. 校验 ffmpeg / ffprobe 可执行。
2. `discover(input_dir)` 产出 pairs 和 orphans。
3. 对 orphan 发送 warn 事件。
4. 用 ffprobe 读取所有图片尺寸，尺寸不一致 fail-fast。
5. 用 ffprobe 读取所有音频时长。
6. `plan(units, options.plan)` 生成 segments。
7. 创建 `.video-gen/build-YYYYMMDD-HHMMSS/`。
8. 串行执行每个 segment 的 ffmpeg 编码。
9. 写 `concat.txt`。
10. 执行 ffmpeg concat。
11. 成功且未 `keep_temp` 时清理临时目录。
12. 返回 `BuildResult`。

失败时保留临时目录供排查。

## 7. 日志事件

公开事件：

```rust
pub enum BuildEvent {
    Discover { units: usize, orphans: usize },
    Warn { message: String },
    Plan { segments: usize, total_ms: u64, summary: String },
    Segment {
        index: usize,
        total: usize,
        name: String,
        kind: SegmentKind,
        basename: Option<String>,
        duration_ms: u64,
        elapsed_ms: u64,
        status: SegmentStatus,
    },
    Concat { output: PathBuf, bytes: u64, duration_ms: u64 },
    Done { output: PathBuf, bytes: u64, duration_ms: u64, elapsed_ms: u64 },
}
```

CLI text/json 输出由 `BuildEvent` 映射而来。Tauri 直接接收结构化事件。

## 8. 错误策略

错误分类：

- `User`：参数非法、输入目录不存在、无配对、basename 冲突、尺寸不一致、ffmpeg/ffprobe 缺失。
- `Runtime`：ffmpeg/ffprobe 执行失败、I/O 失败、JSON 解析失败、输出文件缺失。

CLI 退出码：

- 0：成功
- 1：运行时错误
- 2：用户错误

库调用不退出进程，只返回 `VideoGenError`。

## 9. 依赖

Rust 依赖：

- `clap`：CLI 参数解析
- `serde` / `serde_json`：Tauri 事件和 JSONL 日志
- `thiserror`：错误类型
- `which`：PATH 查找
- `tempfile`：测试临时目录

不引入 async runtime。pipeline 串行执行，库 API 先保持同步，便于 CLI 和 Tauri 后端在阻塞任务中调用。

## 10. 测试迁移

迁移现有 Vitest 覆盖面：

- `discover`：11 个配对/冲突/orphan/排序测试
- `plan`：7 个 timeline 测试
- `ffmpeg_args`：segment argv、concat argv、concat list escaping
- `ffprobe_parse`：音频时长和图片尺寸 JSON 解析
- `output_path`：6 个输出路径解析测试
- `log`：text/quiet/json 输出映射单元测试
- `e2e`：用本机 ffmpeg/ffprobe 构造最小输入并生成 mp4

验证命令：

```bash
cargo fmt --check
cargo test
cargo build --release
```

## 11. Release Workflow 迁移

`.github/workflows/release.yml` 从 Bun 改为 Cargo：

- 使用 `actions-rust-lang/setup-rust-toolchain` 或 runner 自带 Rust toolchain
- `cargo test`
- `cargo build --release`
- 从 `target/release/video-gen[.exe]` 复制二进制
- ffmpeg/ffprobe 仍沿用当前 npm 包方案，使用 Node 脚本解析 `ffmpeg-static` / `ffprobe-static` 并复制到 release tree
- archive 命名和内容保持不变

README 更新为 Cargo 安装和构建命令：

```bash
cargo build --release
./target/release/video-gen build -d input -o output/myvid.mp4
```

## 12. 兼容性判断

这次是整体 Rust 重写，不承诺保留 TypeScript import API、Bun scripts 或 npm package 形态。保留的是用户可见 CLI 行为和 Tauri 可用的 Rust crate API。
