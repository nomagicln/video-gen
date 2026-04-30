# Rust Library + Headless CLI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Bun/TypeScript implementation with a Rust Cargo package that preserves the existing headless CLI behavior and exposes a Tauri-callable Rust library API.

**Architecture:** One Cargo package named `video-gen` builds both `src/lib.rs` and `src/main.rs`. The library owns discovery, planning, ffmpeg/ffprobe integration, logging events, errors, and build orchestration; the CLI is a thin `clap` wrapper around the library.

**Tech Stack:** Rust 2021, `clap`, `serde`, `serde_json`, `thiserror`, `which`, `tempfile`, system or sidecar `ffmpeg`/`ffprobe`.

---

## File Structure

| Path | Action | Responsibility |
|---|---|---|
| `Cargo.toml` | Create | Package metadata, lib/bin targets, dependencies |
| `src/lib.rs` | Create | Public API and module exports |
| `src/error.rs` | Create | `VideoGenError`, exit code mapping |
| `src/log.rs` | Create | `BuildEvent`, `CliLogger`, log modes |
| `src/discover.rs` | Create | Input scanning, basename pairing, orphan detection |
| `src/plan.rs` | Create | Pure timeline planner |
| `src/ffmpeg.rs` | Create | Binary resolution, ffprobe parsing, argv builders, command execution |
| `src/build.rs` | Create | End-to-end build pipeline |
| `src/main.rs` | Create | CLI entry point |
| `tests/*.rs` | Create | Rust replacements for current Vitest suite |
| `README.md` | Modify | Cargo build/use instructions |
| `.github/workflows/release.yml` | Modify | Cargo build/test/release workflow |
| `package.json`, `bun.lock`, `tsconfig.json`, `vitest.config.ts` | Delete | Remove Bun/TypeScript project shape |
| `src/**/*.ts`, `tests/**/*.ts` | Delete | Remove old implementation/tests |

## Task 1: Scaffold Cargo Project

**Files:**
- Create: `Cargo.toml`
- Create: `src/lib.rs`
- Create: `src/error.rs`
- Create: `src/log.rs`
- Replace: `src/main.rs`
- Delete: `src/index.ts`

- [ ] **Step 1: Write failing compile target**

Create `Cargo.toml` with lib/bin targets and dependencies:

```toml
[package]
name = "video-gen"
version = "0.1.0"
edition = "2021"
license = "MIT"

[lib]
name = "video_gen"
path = "src/lib.rs"

[[bin]]
name = "video-gen"
path = "src/main.rs"

[dependencies]
clap = { version = "4.5", features = ["derive"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "2.0"
which = "7.0"

[dev-dependencies]
tempfile = "3.10"
```

- [ ] **Step 2: Add public module shell**

Create `src/lib.rs`:

```rust
pub mod build;
pub mod discover;
pub mod error;
pub mod ffmpeg;
pub mod log;
pub mod plan;

pub use build::{build_video, BinaryOptions, BuildOptions, BuildResult, EncodeOptions};
pub use error::{ErrorKind, VideoGenError};
pub use log::{BuildEvent, SegmentStatus};
pub use plan::{PlanOptions, Segment, SegmentKind, Unit};
```

- [ ] **Step 3: Add minimal error and log types**

Create `src/error.rs` and `src/log.rs` with the public names referenced by `lib.rs`.

- [ ] **Step 4: Run compile and verify it fails because referenced modules are missing**

Run: `cargo test`

Expected: compiler errors for missing `build`, `discover`, `ffmpeg`, or `plan` modules.

- [ ] **Step 5: Add minimal module files that compile**

Add minimal modules with the exported type names only. Later tasks fill each module with the tested production behavior.

- [ ] **Step 6: Run compile**

Run: `cargo test`

Expected: compile succeeds before behavior tests are added.

## Task 2: Port Planner with TDD

**Files:**
- Create: `src/plan.rs`
- Create: `tests/plan.rs`

- [ ] **Step 1: Write failing planner tests**

Port the seven tests from `tests/pipeline/plan.test.ts` into `tests/plan.rs`, covering empty input, zero padding, full padding, lead-in first image, tail last image, gap previous image, and negative duration rejection.

- [ ] **Step 2: Run planner tests and verify RED**

Run: `cargo test --test plan`

Expected: tests fail because `plan_segments` is not implemented.

- [ ] **Step 3: Implement planner**

Implement:

```rust
pub fn plan_segments(units: &[Unit], opts: &PlanOptions) -> Result<Vec<Segment>, VideoGenError>
```

Rules match the approved design: optional lead-in, unit per input, optional gap between adjacent units, optional tail.

- [ ] **Step 4: Run planner tests and verify GREEN**

Run: `cargo test --test plan`

Expected: all planner tests pass.

## Task 3: Port Discovery with TDD

**Files:**
- Create: `src/discover.rs`
- Create: `tests/discover.rs`

- [ ] **Step 1: Write failing discovery tests**

Port the eleven tests from `tests/pipeline/discover.test.ts`, using `tempfile::TempDir` and helper `touch(path)`.

- [ ] **Step 2: Run discovery tests and verify RED**

Run: `cargo test --test discover`

Expected: tests fail because `discover` is not implemented.

- [ ] **Step 3: Implement discovery**

Implement scan of root, `images`, and `audio`; supported extensions; duplicate detection; orphan reporting; lexicographic pair/orphan sorting; `User` errors for missing input or no pairs.

- [ ] **Step 4: Run discovery tests and verify GREEN**

Run: `cargo test --test discover`

Expected: all discovery tests pass.

## Task 4: Port Output Path and Build Directory Helpers

**Files:**
- Create or modify: `src/build.rs`
- Create: `tests/output_path.rs`

- [ ] **Step 1: Write failing output path tests**

Port six `resolveOutputPath` cases from `tests/bin/headless.test.ts`.

- [ ] **Step 2: Run tests and verify RED**

Run: `cargo test --test output_path`

Expected: tests fail because `resolve_output_path` is not implemented.

- [ ] **Step 3: Implement helpers**

Implement:

```rust
pub fn resolve_output_path(raw_output: Option<&Path>, input_dir: &Path) -> PathBuf
pub fn make_run_id(now: SystemTime) -> String
pub struct BuildDir
```

- [ ] **Step 4: Run tests and verify GREEN**

Run: `cargo test --test output_path`

Expected: output path tests pass.

## Task 5: Port ffmpeg/ffprobe Pure Logic with TDD

**Files:**
- Create: `src/ffmpeg.rs`
- Create: `tests/ffprobe_parse.rs`
- Create: `tests/ffmpeg_args.rs`

- [ ] **Step 1: Write failing ffprobe parse tests**

Port tests for `parseAudioDurationMs` and `parseImageSize`.

- [ ] **Step 2: Write failing argv tests**

Port tests for unit segment argv, silent segment argv, concat argv, and concat list escaping.

- [ ] **Step 3: Run tests and verify RED**

Run: `cargo test --test ffprobe_parse --test ffmpeg_args`

Expected: tests fail because parser and argv functions are not implemented.

- [ ] **Step 4: Implement parser and argv builders**

Implement:

```rust
pub fn parse_audio_duration_ms(json_text: &str) -> Result<u64, VideoGenError>
pub fn parse_image_size(json_text: &str) -> Result<ImageSize, VideoGenError>
pub fn segment_argv(segment: &Segment, output: &Path, opts: &EncodeOptions) -> Vec<String>
pub fn concat_argv(list: &Path, output: &Path) -> Vec<String>
pub fn concat_list_content(segment_filenames: &[String]) -> String
```

- [ ] **Step 5: Run tests and verify GREEN**

Run: `cargo test --test ffprobe_parse --test ffmpeg_args`

Expected: parser and argv tests pass.

## Task 6: Implement Build Pipeline with E2E

**Files:**
- Modify: `src/build.rs`
- Modify: `src/ffmpeg.rs`
- Create: `tests/e2e.rs`

- [ ] **Step 1: Write failing e2e test**

Create a fixture test that uses ffmpeg to generate two small images and two short audio files, calls `build_video`, and asserts output exists and is non-empty.

- [ ] **Step 2: Run e2e test and verify RED**

Run: `cargo test --test e2e -- --nocapture`

Expected: fails because `build_video` does not run the pipeline yet.

- [ ] **Step 3: Implement binary resolution and process execution**

Implement explicit binary path, env var, sibling binary, and PATH lookup resolution. Implement `check_binary`, `probe_audio_duration_ms`, `probe_image_size`, and `run_ffmpeg`.

- [ ] **Step 4: Implement `build_video` orchestration**

Wire discovery, validation, planning, segment encoding, concat list writing, concat execution, event emission, success cleanup, and failure temp retention.

- [ ] **Step 5: Run e2e test and verify GREEN**

Run: `cargo test --test e2e -- --nocapture`

Expected: e2e test passes and creates a valid mp4.

## Task 7: Implement CLI Compatibility

**Files:**
- Modify: `src/main.rs`
- Create: `tests/cli.rs`

- [ ] **Step 1: Write failing CLI tests**

Add tests for CLI parsing helpers where practical: seconds parsing, integer bounds, and json/text logger event formatting.

- [ ] **Step 2: Run CLI tests and verify RED**

Run: `cargo test --test cli`

Expected: tests fail because CLI helpers are missing.

- [ ] **Step 3: Implement CLI**

Use `clap` derive or builder API to support `video-gen build` and all existing flags. Map `VideoGenError::exit_code()` to process exit code.

- [ ] **Step 4: Run CLI tests and help smoke**

Run:

```bash
cargo test --test cli
cargo run -- build --help
```

Expected: tests pass and help shows the `build` command flags.

## Task 8: Replace Project Metadata and Docs

**Files:**
- Delete: `package.json`
- Delete: `bun.lock`
- Delete: `tsconfig.json`
- Delete: `vitest.config.ts`
- Delete: old `src/**/*.ts`
- Delete: old `tests/**/*.ts`
- Modify: `README.md`
- Modify: `.github/workflows/release.yml`

- [ ] **Step 1: Remove TypeScript/Bun files**

Delete Node/Bun project files and old TS implementation/tests after Rust tests cover the behavior.

- [ ] **Step 2: Update README**

Replace setup commands with Cargo commands and keep CLI usage/release instructions.

- [ ] **Step 3: Update release workflow**

Replace Bun test/build steps with Cargo test/build steps. Keep archive names unchanged, but do not include ffmpeg/ffprobe in release archives.

- [ ] **Step 4: Run full verification**

Run:

```bash
cargo fmt --check
cargo test
cargo build --release
./target/release/video-gen --help
```

Expected: all commands exit 0.

## Self-Review Checklist

- Spec coverage: API, CLI, ffmpeg resolution, errors, events, tests, release workflow are covered by tasks.
- Red-flag scan: no task uses unresolved-marker or deferred-work wording.
- Type consistency: `BuildOptions`, `BuildEvent`, `VideoGenError`, `PlanOptions`, `EncodeOptions`, `BinaryOptions` names match the design doc.
