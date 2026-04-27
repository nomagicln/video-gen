# video-gen

Headless CLI: pair images + audio by basename, build one mp4 with optional lead-in / gap / tail silences.

## Usage

```bash
video-gen build -d input -o output/myvid.mp4 --lead-in 2 --tail 2 --gap 0.5
```

Inputs go in `input/` (or `input/images/` + `input/audio/`); each image must have an audio file with the same basename:

```
input/
  01_intro.jpg   01_intro.mp3
  02_demo.jpg    02_demo.mp3
```

Mismatched basenames are skipped with a `WARN`. Multiple files for the same basename, or unequal image dimensions, fail-fast.

Outputs h264/aac mp4; defaults `--fps 30 --crf 20 --preset medium --audio-bitrate 192k`. See `video-gen build --help` for the full flag list.

`--quiet` suppresses progress; `--json` switches to one-JSON-per-line for CI.

## Setup

```bash
bun install
bun run package   # → dist/video-gen
```

ffmpeg + ffprobe must be on `$PATH`, beside the binary, or pointed at by `VIDEO_GEN_FFMPEG` / `VIDEO_GEN_FFPROBE`. Release tarballs bundle both.

## Releases

Pre-built archives for darwin-arm64, linux-x64, windows-x64 are published on the [Releases page](https://github.com/nomagicln/video-gen/releases). Each archive bundles `video-gen` + `ffmpeg` + `ffprobe` — extract and run, no separate ffmpeg install required.

```bash
tar -xzf video-gen-darwin-arm64.tar.gz
cd video-gen-darwin-arm64
./video-gen build -d ../my-input -o ../out.mp4 --lead-in 1 --gap 0.5
```

The bundled binaries are not signed or notarized. On first run, macOS may block them as "from an unidentified developer". Remove the quarantine attribute once:

```bash
xattr -d com.apple.quarantine ./video-gen ./ffmpeg ./ffprobe
```

Linux and Windows do not need this step.

Verify your download with the matching `.sha256` file:

```bash
shasum -a 256 -c video-gen-darwin-arm64.tar.gz.sha256
```
