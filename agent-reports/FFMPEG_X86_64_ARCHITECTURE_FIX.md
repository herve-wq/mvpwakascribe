# FFmpeg & Sidecar Architecture Fix — Intel (x86_64) macOS Support

**Date**: 2025-02-13
**Severity**: Critical — app non-functional on Intel Macs
**Platforms affected**: macOS x86_64 (Intel)

---

## Symptom

When running VoiceTypr on an Intel Mac, audio recording and file transcription fail with:

```
Audio normalization (ffmpeg) failed: Failed to spawn
'/Applications/Voicetypr.app/Contents/MacOS/ffmpeg': Bad CPU type in executable (os error 86)
```

The error `Bad CPU type in executable` (EBADARCH) means the bundled binary is compiled for a different CPU architecture than the host.

---

## Root Cause Analysis

### 1. `ensure-ffmpeg-sidecar.cjs` — ARM64-only downloads

The script downloads ffmpeg/ffprobe exclusively from ARM64 sources, regardless of the host architecture:

```js
// Line 117-118 — always ARM64
const ffmpegZipUrl = process.env.FFMPEG_MAC_URL || 'https://www.osxexperts.net/ffmpeg80arm.zip';
const ffprobeZipUrl = process.env.FFPROBE_MAC_URL || 'https://www.osxexperts.net/ffprobe80arm.zip';
```

The script detects non-ARM hosts but only logs a warning and continues anyway:

```js
// Line 103-105
if (process.arch !== 'arm64') {
  console.warn('[ensure-ffmpeg-sidecar] Non-arm64 mac detected; this project targets Apple Silicon.');
}
```

Symlinks are created only for aarch64:

```js
// Line 110-111
ensureSymlink(ffmpegDst, path.join(distDir, 'ffmpeg-aarch64-apple-darwin'));
ensureSymlink(ffprobeDst, path.join(distDir, 'ffprobe-aarch64-apple-darwin'));
```

### 2. `src-tauri/src/ffmpeg/mod.rs` — No x86_64 binary candidates

The runtime binary resolution only searches for ARM64-named binaries on macOS:

```rust
// macOS candidates
["ffmpeg", "ffmpeg-aarch64-apple-darwin"]
["ffprobe", "ffprobe-aarch64-apple-darwin"]
```

There is no `ffmpeg-x86_64-apple-darwin` candidate, so even if an Intel binary were present, it would not be found by name.

### 3. `build-parakeet-sidecar.sh` — Forces ARM64 build

The Swift Parakeet sidecar is always built for ARM64:

```bash
# Line 21-22
swift build -c release --arch arm64
```

Tauri's `build.rs` then creates an x86_64-named copy, but the underlying binary is still ARM64 and will crash on Intel.

### 4. `src-tauri/build.rs` — Masks the problem

The build script creates a copy named `parakeet-sidecar-x86_64-apple-darwin` from the ARM64 binary, which lets the build pass but produces a non-functional binary on Intel.

---

## Impact

| Component | Status on Intel Mac |
|---|---|
| ffmpeg (audio normalization) | **Broken** — EBADARCH |
| ffprobe (audio probing) | **Broken** — EBADARCH |
| Parakeet sidecar | **Broken** — ARM64 binary disguised as x86_64 |
| Whisper transcription (Rust-native) | OK — compiled for host arch by Cargo |
| Frontend / UI | OK |

**Result**: No transcription possible on Intel Macs (neither recording nor file upload).

---

## Proposed Fix

### Fix 1 — `scripts/ensure-ffmpeg-sidecar.cjs`

Detect `process.arch` and download the correct binaries:

- `arm64` → download from `osxexperts.net/ffmpeg80arm.zip` (current behavior)
- `x64` → download from an Intel source (e.g. `osxexperts.net/ffmpeg80intel.zip` or `evermeet.cx/ffmpeg`)
- Create symlinks matching the host triple:
  - `ffmpeg-aarch64-apple-darwin` for ARM64
  - `ffmpeg-x86_64-apple-darwin` for Intel
- Add SHA256 checksums for Intel binaries (hardcoded or via env vars)

### Fix 2 — `src-tauri/src/ffmpeg/mod.rs`

Add x86_64 candidates to the binary resolution lists:

```rust
// macOS candidates (both architectures)
#[cfg(target_os = "macos")]
const FFMPEG_NAMES: &[&str] = &["ffmpeg", "ffmpeg-aarch64-apple-darwin", "ffmpeg-x86_64-apple-darwin"];

#[cfg(target_os = "macos")]
const FFPROBE_NAMES: &[&str] = &["ffprobe", "ffprobe-aarch64-apple-darwin", "ffprobe-x86_64-apple-darwin"];
```

### Fix 3 — `scripts/build-parakeet-sidecar.sh`

Build for the host architecture instead of forcing ARM64:

```bash
# Detect host arch
HOST_ARCH=$(uname -m)  # x86_64 or arm64
swift build -c release --arch "$HOST_ARCH"
```

Name the output binary with the correct triple:

```bash
if [[ "$HOST_ARCH" == "arm64" ]]; then
  TRIPLE="aarch64-apple-darwin"
else
  TRIPLE="x86_64-apple-darwin"
fi
cp "$SRC_BIN_PATH" "dist/parakeet-sidecar-$TRIPLE"
```

### Fix 4 — `src-tauri/build.rs`

Use `std::env::var("CARGO_CFG_TARGET_ARCH")` to verify the sidecar binary matches the actual build target, instead of blindly aliasing.

---

## Files to Modify

| File | Change |
|---|---|
| `scripts/ensure-ffmpeg-sidecar.cjs` | Add x64 macOS download source + symlinks |
| `src-tauri/src/ffmpeg/mod.rs` | Add `x86_64-apple-darwin` binary candidates |
| `scripts/build-parakeet-sidecar.sh` | Build for host arch, not hardcoded ARM64 |
| `src-tauri/build.rs` | Validate sidecar arch matches target |

**No changes needed for Windows** — already functional with x86_64 binaries.

---

## Notes

- The Whisper engine (compiled via Cargo as Rust native code) works correctly on both architectures because Cargo compiles for the host target.
- A universal binary (fat binary) approach for ffmpeg is possible but would double the app size (~100MB for ffmpeg+ffprobe). Per-arch downloads are preferred.
- FluidAudio (Swift Parakeet dependency) may or may not support x86_64 — this needs verification before assuming Fix 3 will work.
