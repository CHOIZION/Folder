# HEART architecture

HEART is a Windows Tauri application with an Android remote client. The remote
pipeline follows one rule: live interaction never waits behind stale video or
pointer events.

## Component boundaries

- `src/` owns desktop presentation and typed Tauri command calls.
- `src-tauri/src/core`, `db`, and `commands` own scanning, persistence, and
  process launching. UI code performs no direct database work.
- `src-tauri/src/remote.rs` owns HTTP routing, pairing, authorization, and
  library endpoints.
- `src-tauri/src/remote/streaming.rs` owns pacing, MJPEG adaptation, the H.264
  wire format, and stream backpressure.
- `src-tauri/src/desktop.rs` owns Windows capture and input injection.
- `src-tauri/src/h264.rs` owns Media Foundation setup and Annex-B output.
- `H264StreamClient` owns Android transport and MediaCodec state.
- `H264FullscreenController` owns the native Surface and controls.
- `RemoteGestureController` owns gesture interpretation.
- `RemoteInputDispatcher` owns input coalescing and delivery.

## Streaming invariants

1. There is no video frame queue. A slow socket blocks the producer; old frames
   are never accumulated in memory.
2. Only one transport owns capture. Android closes MJPEG before opening native
   H.264, while MJPEG remains available as the compatibility fallback.
3. Capture, previous-frame, NV12, JPEG, and Android packet buffers are reused.
4. Exact memory comparison detects unchanged frames. Static desktops consume no
   encoder bandwidth beyond bounded keepalives.
5. H.264 keyframes carry SPS/PPS and are requested at most once per second.
6. Pointer moves are latest-value state, not a FIFO. Wheel deltas are bounded
   and accumulated; clicks and keys remain ordered control messages.
7. Background transitions release the connection, MediaCodec, Surface, and
   capture ownership. Resume creates fresh decoder state.

## Error strategy

Expected environmental failures return user-facing messages at the process
boundary. Hardware encoder failure falls back to Microsoft's software MFT; an
unusable H.264 path falls back to MJPEG. Reconnect delays use capped exponential
backoff.

## Verification

`npm run verify` runs JavaScript tests, Svelte/TypeScript diagnostics, Rust
format checking, Clippy with warnings denied, and Rust tests. Windows CI also
builds the web frontend and compiles/signs an Android test APK.
