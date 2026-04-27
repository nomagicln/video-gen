# video-gen Release Pipeline Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the multi-platform GitHub Actions release workflow defined in `docs/superpowers/specs/2026-04-27-release-pipeline-design.md` — `git push v0.1.0` produces a 4-platform set of tarballs/zip on GitHub Release, each bundling `video-gen` + `ffmpeg` + `ffprobe` + `README.md` + `LICENSE`.

**Architecture:** One workflow file (`.github/workflows/release.yml`) with two jobs. `build` runs across a 4-target matrix (darwin-arm64, linux-x64, linux-arm64, windows-x64) — install deps, bundle ffmpeg/ffprobe via `ffmpeg-static` + `ffprobe-static` npm packages, run typecheck + full test suite (e2e included, pointed at the bundled binaries), compile the bun single-file binary, smoke-test, archive, upload as artifact. `release` job (gated on tag push or dispatch-with-tag) downloads all artifacts and uploads them via `softprops/action-gh-release@v2` with auto-generated release notes.

**Tech Stack:** GitHub Actions, `oven-sh/setup-bun@v2`, `ffmpeg-static@^5.2.0`, `ffprobe-static@^3.1.0`, `softprops/action-gh-release@v2`, bash + pwsh.

---

## File Structure

| Path | Action | Responsibility |
|---|---|---|
| `LICENSE` | create | MIT license text, copyright `nomagicln` |
| `.gitignore` | modify | add `dist/` so local `bun run package` output isn't tracked |
| `README.md` | modify | add "Releases" section with `xattr -d` instruction for macOS |
| `.github/workflows/release.yml` | create | the full workflow |

No source code changes needed — `tests/e2e/fixtures.ts` already routes through `resolveBinary` (`src/ffmpeg/resolve.ts`), so setting `VIDEO_GEN_FFMPEG` / `VIDEO_GEN_FFPROBE` makes e2e use the bundled binaries.

Validation has no automated test layer — it's verified by:
1. dispatch dry-run (artifacts only, no release) on a feature branch
2. real `v0.0.0-test` tag-push as a soak run (delete the test release/tag after)
3. real `v0.1.0` tag-push as the actual first release

---

## Task 1: Add MIT LICENSE file

**Files:**
- Create: `LICENSE`

- [ ] **Step 1: Write `LICENSE`**

```
MIT License

Copyright (c) 2026 nomagicln

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

- [ ] **Step 2: Commit**

```bash
git add LICENSE
git commit -m "docs: add MIT LICENSE"
```

---

## Task 2: Ignore `dist/`

**Files:**
- Modify: `.gitignore`

- [ ] **Step 1: Verify `dist/` is not already ignored**

Run: `cat .gitignore`
Expected: shows current contents — should include `node_modules/`, `output/*`, `input/*`, `.video-gen/`, etc., but NOT `dist/`.

- [ ] **Step 2: Append `dist/` to `.gitignore`**

The current `.gitignore` (verified above) starts:

```
node_modules/
dist/
.env
...
```

If `dist/` is already there, this task is a no-op — skip the rest. If not, add the line. Final `.gitignore` should look like (showing first ~5 lines):

```
node_modules/
dist/
.env
.env.local
output/*
```

- [ ] **Step 3: Verify dist/ is not currently tracked**

Run: `git ls-files dist/`
Expected: empty output. If anything shows up, run `git rm -r --cached dist/` to untrack (a separate stage from .gitignore).

- [ ] **Step 4: Commit (if changed)**

```bash
git add .gitignore
git commit -m "chore: gitignore dist/"
```

If `.gitignore` was already correct, skip the commit and move on.

---

## Task 3: Add README "Releases" section

**Files:**
- Modify: `README.md`

The current README (after the existing "Setup" section that talks about `bun run package`) needs a new section explaining the release tarball flow + macOS Gatekeeper workaround.

- [ ] **Step 1: Read the current README**

Run: `cat README.md`
Expected: shows existing content with `## Usage` and `## Setup` sections.

- [ ] **Step 2: Append a new section to `README.md`**

Add this at the end of the file:

```markdown

## Releases

Pre-built archives for darwin-arm64, linux-x64, linux-arm64, windows-x64 are published on the [Releases page](https://github.com/nomagicln/video-gen/releases). Each archive bundles `video-gen` + `ffmpeg` + `ffprobe` — extract and run, no separate ffmpeg install required.

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
```

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: README releases section + macOS quarantine notes"
```

---

## Task 4: Create the release workflow file

**Files:**
- Create: `.github/workflows/release.yml`

This is the meat of the work. It's one file, but long — write it in one go.

- [ ] **Step 1: Create `.github/workflows/release.yml`**

```yaml
name: release

on:
  push:
    tags:
      - 'v*'
  workflow_dispatch:
    inputs:
      tag:
        description: 'tag name to publish under (e.g. v0.1.0). leave empty for dry-run (artifacts only).'
        required: false
        default: ''

permissions:
  contents: write

jobs:
  build:
    name: build (${{ matrix.target }})
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        include:
          - target: darwin-arm64
            os: macos-14
            archive: tar.gz
            ext: ''
          - target: linux-x64
            os: ubuntu-22.04
            archive: tar.gz
            ext: ''
          - target: linux-arm64
            os: ubuntu-24.04-arm
            archive: tar.gz
            ext: ''
          - target: windows-x64
            os: windows-latest
            archive: zip
            ext: '.exe'

    steps:
      - uses: actions/checkout@v4

      - uses: oven-sh/setup-bun@v2
        with:
          bun-version: latest

      - name: install deps
        run: bun install --frozen-lockfile

      - name: bundle ffmpeg + ffprobe
        shell: bash
        run: |
          bun add ffmpeg-static@^5.2.0 ffprobe-static@^3.1.0
          mkdir -p dist
          bun -e "
          const fs = require('fs');
          const ffmpegPath = require('ffmpeg-static');
          const ffprobePkg = require('ffprobe-static');
          const ffprobePath = ffprobePkg.path;
          if (!ffmpegPath) throw new Error('ffmpeg-static did not resolve a binary for this platform');
          if (!ffprobePath) throw new Error('ffprobe-static did not resolve a binary for this platform');
          const ext = '${{ matrix.ext }}';
          fs.copyFileSync(ffmpegPath, 'dist/ffmpeg' + ext);
          fs.copyFileSync(ffprobePath, 'dist/ffprobe' + ext);
          if (process.platform !== 'win32') {
            fs.chmodSync('dist/ffmpeg' + ext, 0o755);
            fs.chmodSync('dist/ffprobe' + ext, 0o755);
          }
          console.log('bundled', ffmpegPath, '->', 'dist/ffmpeg' + ext);
          console.log('bundled', ffprobePath, '->', 'dist/ffprobe' + ext);
          "

      - name: typecheck
        run: bun run typecheck

      - name: test (with bundled ffmpeg/ffprobe)
        if: matrix.target != 'windows-x64'
        shell: bash
        env:
          VIDEO_GEN_FFMPEG: ${{ github.workspace }}/dist/ffmpeg
          VIDEO_GEN_FFPROBE: ${{ github.workspace }}/dist/ffprobe
        run: bun run test

      - name: test (with bundled ffmpeg/ffprobe, windows)
        if: matrix.target == 'windows-x64'
        shell: pwsh
        run: |
          $env:VIDEO_GEN_FFMPEG = "$env:GITHUB_WORKSPACE\dist\ffmpeg.exe"
          $env:VIDEO_GEN_FFPROBE = "$env:GITHUB_WORKSPACE\dist\ffprobe.exe"
          bun run test

      - name: compile binary
        shell: bash
        run: |
          bun build src/index.ts --compile --minify --outfile dist/video-gen${{ matrix.ext }}

      - name: smoke test
        if: matrix.target != 'windows-x64'
        shell: bash
        run: |
          ./dist/video-gen --help | head -20
          ./dist/ffmpeg -version | head -1
          ./dist/ffprobe -version | head -1

      - name: smoke test (windows)
        if: matrix.target == 'windows-x64'
        shell: pwsh
        run: |
          .\dist\video-gen.exe --help | Select-Object -First 20
          .\dist\ffmpeg.exe -version | Select-Object -First 1
          .\dist\ffprobe.exe -version | Select-Object -First 1

      - name: stage release tree
        shell: bash
        run: |
          name="video-gen-${{ matrix.target }}"
          mkdir -p "release/${name}"
          cp "dist/video-gen${{ matrix.ext }}" "release/${name}/"
          cp "dist/ffmpeg${{ matrix.ext }}" "release/${name}/"
          cp "dist/ffprobe${{ matrix.ext }}" "release/${name}/"
          cp README.md "release/${name}/"
          cp LICENSE "release/${name}/"
          ls -la "release/${name}"

      - name: archive (tar.gz)
        if: matrix.archive == 'tar.gz'
        shell: bash
        run: |
          name="video-gen-${{ matrix.target }}"
          tar -C release -czf "release/${name}.tar.gz" "${name}"
          (cd release && shasum -a 256 "${name}.tar.gz" > "${name}.tar.gz.sha256")

      - name: archive (zip)
        if: matrix.archive == 'zip'
        shell: pwsh
        run: |
          $name = "video-gen-${{ matrix.target }}"
          Compress-Archive -Path "release/$name/*" -DestinationPath "release/$name.zip"
          $hash = (Get-FileHash -Algorithm SHA256 "release/$name.zip").Hash.ToLower()
          "$hash  $name.zip" | Out-File -Encoding ascii "release/$name.zip.sha256"

      - uses: actions/upload-artifact@v4
        with:
          name: video-gen-${{ matrix.target }}
          path: |
            release/video-gen-${{ matrix.target }}.${{ matrix.archive }}
            release/video-gen-${{ matrix.target }}.${{ matrix.archive }}.sha256
          if-no-files-found: error

  release:
    name: publish release
    needs: build
    runs-on: ubuntu-latest
    if: startsWith(github.ref, 'refs/tags/v') || (github.event_name == 'workflow_dispatch' && github.event.inputs.tag != '')
    steps:
      - uses: actions/download-artifact@v4
        with:
          path: assets
          merge-multiple: true

      - name: list assets
        run: ls -la assets

      - uses: softprops/action-gh-release@v2
        with:
          tag_name: ${{ github.event.inputs.tag || github.ref_name }}
          files: assets/*
          generate_release_notes: true
          draft: false
          prerelease: false
```

- [ ] **Step 2: Validate workflow syntax with `gh`**

If the `gh` CLI is available, syntax-check the file:

Run: `gh workflow view release.yml 2>/dev/null || cat .github/workflows/release.yml | head -5`
Expected: either `gh workflow view` shows the workflow info (only after push), or the cat fallback shows the first 5 lines. The point is just to confirm the file is non-empty and parseable. (Real syntax errors will only surface on push; that's expected.)

- [ ] **Step 3: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: 4-platform release pipeline (build matrix + auto release)"
```

---

## Task 5: Push branch and trigger dry-run

**Files:** none (only git + GitHub UI)

- [ ] **Step 1: Push the branch**

Run:
```bash
git push -u origin feat/release-pipeline
```
Expected: branch pushed, GitHub URL printed.

- [ ] **Step 2: Trigger workflow_dispatch dry-run**

Open the GitHub Actions tab for the repo:
```
https://github.com/nomagicln/video-gen/actions/workflows/release.yml
```

Click **Run workflow**. Pick branch `feat/release-pipeline`. Leave the `tag` field **empty**. Click run.

Expected outcome:
- 4 `build` jobs run in parallel
- All 4 succeed (typecheck + test passing on each runner is the main risk)
- `release` job shows as **skipped** (because tag input is empty and ref is not `refs/tags/v*`)
- Each build job's "Summary" page shows an artifact: `video-gen-<target>` containing the archive + .sha256

If anything fails: read the failing step's logs, fix locally, re-push, re-run dispatch. Don't proceed until all 4 builds are green.

- [ ] **Step 3: Download an artifact and sanity-check it**

From the workflow run summary, download the `video-gen-darwin-arm64` artifact (a zip containing the tar.gz + sha256). Extract it locally:

```bash
cd /tmp
unzip -o ~/Downloads/video-gen-darwin-arm64.zip
shasum -a 256 -c video-gen-darwin-arm64.tar.gz.sha256
tar -xzf video-gen-darwin-arm64.tar.gz
cd video-gen-darwin-arm64
ls -la
```

Expected: `ls` shows `video-gen`, `ffmpeg`, `ffprobe`, `README.md`, `LICENSE`. shasum verifies. **If running on macOS, you'll need to clear quarantine first** (the README explains):
```bash
xattr -d com.apple.quarantine ./video-gen ./ffmpeg ./ffprobe
./video-gen --help
```
Expected: help text matching the local CLI.

---

## Task 6: Merge to main

**Files:** none (git operations only)

- [ ] **Step 1: Verify main is clean and we're branched off the right place**

Run:
```bash
git fetch origin
git log --oneline origin/main..HEAD
```
Expected: lists exactly the 4 commits from this plan (LICENSE, dist/ gitignore, README, workflow) plus the spec commit from brainstorming.

- [ ] **Step 2: Switch to main and merge**

```bash
git checkout main
git merge --no-ff feat/release-pipeline -m "Merge feat/release-pipeline: GitHub Actions release pipeline"
```
Expected: clean merge, no conflicts.

- [ ] **Step 3: Push main**

```bash
git push origin main
```

---

## Task 7: First real release (`v0.1.0`)

**Files:** none

This task IS the test of the workflow. If anything breaks, fix forward via Task 8.

- [ ] **Step 1: Confirm tests pass locally one more time**

Run:
```bash
PATH="$HOME/.bun/bin:/opt/homebrew/bin:$PATH" ~/.bun/bin/bun run test
```
Expected: all 52 tests passing.

- [ ] **Step 2: Tag and push**

```bash
git tag v0.1.0
git push origin v0.1.0
```

- [ ] **Step 3: Watch the workflow run**

Open `https://github.com/nomagicln/video-gen/actions`. The push triggers a new `release` workflow run. Watch:
- 4 `build` jobs run (~3-5 minutes each)
- `release` job runs after they all finish (~30 seconds)

Expected on success:
- Visit `https://github.com/nomagicln/video-gen/releases/tag/v0.1.0`
- See 8 files (4 archives + 4 sha256)
- Auto-generated release notes from commit history since repo creation

- [ ] **Step 4: End-to-end sanity check**

From the release page, download `video-gen-darwin-arm64.tar.gz` (or the platform you're on). Verify the same way as Task 5 step 3:

```bash
cd /tmp
rm -rf video-gen-test && mkdir video-gen-test && cd video-gen-test
curl -L -o video-gen-darwin-arm64.tar.gz \
  https://github.com/nomagicln/video-gen/releases/download/v0.1.0/video-gen-darwin-arm64.tar.gz
curl -L -o video-gen-darwin-arm64.tar.gz.sha256 \
  https://github.com/nomagicln/video-gen/releases/download/v0.1.0/video-gen-darwin-arm64.tar.gz.sha256
shasum -a 256 -c video-gen-darwin-arm64.tar.gz.sha256
tar -xzf video-gen-darwin-arm64.tar.gz
cd video-gen-darwin-arm64
xattr -d com.apple.quarantine ./video-gen ./ffmpeg ./ffprobe 2>/dev/null || true
./video-gen --help
```
Expected: shasum verifies. `./video-gen --help` prints CLI usage.

Then a real one-segment build:

```bash
mkdir -p smoke/input
./ffmpeg -y -f lavfi -i "color=c=red:s=320x180:d=1" -frames:v 1 smoke/input/01.png
./ffmpeg -y -f lavfi -i "anullsrc=r=44100:cl=stereo" -t 1 smoke/input/01.wav
VIDEO_GEN_FFMPEG=$PWD/ffmpeg VIDEO_GEN_FFPROBE=$PWD/ffprobe \
  ./video-gen build -d smoke/input -o smoke/out.mp4 --lead-in 1 --tail 1
./ffprobe -v error -of default=nw=1:nk=1 -show_entries format=duration smoke/out.mp4
```
Expected: build prints discover/plan/build phases. ffprobe prints something near `3.0` (lead 1 + unit 1 + tail 1).

If everything checks out: v0.1.0 is shipped.

---

## Task 8: If the first release fails — recover

**Files:** none

This is documentation for the engineer to follow IF Task 7 fails partway. It's not a step to execute upfront; only do it if a build fails OR the publish job fails.

- [ ] **Step 1: Diagnose**

Read the failing job's logs in the Actions tab. Common failures and fixes:

| Symptom | Likely cause | Fix |
|---|---|---|
| `bun install --frozen-lockfile` fails on a runner | bun.lock changed since last push | run `bun install` locally, commit the new lockfile, push, re-tag |
| `ffmpeg-static` / `ffprobe-static` resolves to null on `linux-arm64` | upstream npm package missing arm64 binary | switch that platform to a different bundling source; out of v0.1 scope — pin or skip linux-arm64 |
| e2e test fails on a specific runner | encoder version difference between local + CI ffmpeg-static | widen the duration tolerance in `tests/e2e/build.test.ts`, or skip e2e on that platform with `it.skipIf` (last resort) |
| `softprops/action-gh-release` 403 | token permissions missing | confirm `permissions: contents: write` is at workflow level (it is, in our file) |

- [ ] **Step 2: Delete the broken tag + release on GitHub**

```bash
# delete the tag locally and remotely
git tag -d v0.1.0
git push origin :refs/tags/v0.1.0
```

Then on GitHub UI: Releases → v0.1.0 → Delete release.

- [ ] **Step 3: Fix the underlying issue, commit on `main`, re-tag**

```bash
# (after fixing whatever broke)
git tag v0.1.0
git push origin v0.1.0
```

This re-runs the workflow. Repeat Task 7 step 4 to verify.

---

## Self-review

**Spec coverage**

| Spec section | Implementing task |
|---|---|
| §2 Platform matrix (4 targets) | Task 4 (matrix block) |
| §3 Triggers (tag + dispatch dry-run) | Task 4 (`on:` block + `release` job `if:` guard) |
| §4 ffmpeg/ffprobe via npm | Task 4 ("bundle ffmpeg + ffprobe" step) |
| §5 Tarball contents | Task 4 ("stage release tree" step) |
| §5 archive naming + sha256 | Task 4 (archive steps) |
| §6 LICENSE | Task 1 + Task 4 (cp into tarball) |
| §7 README quarantine note | Task 3 |
| §8 CI runs full test with bundled ffmpeg | Task 4 (test step with `VIDEO_GEN_FFMPEG`/`FFPROBE` env) |
| §9 Workflow steps | Task 4 (entire file) |
| §10 `dist/` ignored | Task 2 |
| §11 Verification (dispatch dry-run, real tag) | Task 5, Task 7 |
| §12 Failure recovery | Task 8 |

All sections covered.

**Placeholder scan**: zero "TBD/TODO". Each step has full content.

**Consistency**:
- Archive name `video-gen-<target>.tar.gz` / `.zip` consistent across Tasks 4, 5, 7.
- Env var names `VIDEO_GEN_FFMPEG` / `VIDEO_GEN_FFPROBE` match what `src/ffmpeg/resolve.ts` reads (verified earlier in v0.1 implementation).
- `softprops/action-gh-release@v2` and `actions/download-artifact@v4` versions consistent with voice-gen's working workflow.
- Branch name `feat/release-pipeline` consistent in Tasks 5 + 6 (this branch was created in brainstorming).

No issues found.

---

## Plan summary

8 tasks. Tasks 1-4 are the actual file changes (~15 minutes total). Task 5 is the first dry-run on GitHub (~5 minutes wall-clock for runner). Task 6 is the merge. Task 7 is the first real tagged release. Task 8 only runs if Task 7 fails.

After Task 7 ships v0.1.0, the spec is fully implemented.
