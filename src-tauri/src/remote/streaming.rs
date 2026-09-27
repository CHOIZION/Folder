use std::{
    fmt::Write as _,
    io::Write,
    net::TcpStream,
    thread,
    time::{Duration, Instant},
};

use crate::{desktop, h264};

use super::{write_json_error, RemoteHostState};

pub(super) const MJPEG_WIDTH: i32 = 1_080;
pub(super) const MJPEG_QUALITY: u8 = 52;

const FRAME_INTERVAL: Duration = Duration::from_micros(16_667);
const MJPEG_IDLE_REFRESH: Duration = Duration::from_millis(300);
const MJPEG_SLOW_FRAME: Duration = Duration::from_millis(24);
const MJPEG_BOUNDARY: &str = "heart-frame";
const H264_WIDTH: i32 = 1_728;
const H264_FPS: u32 = 60;
const H264_LAN_BITRATE: u32 = 18_000_000;
const H264_TAILSCALE_BITRATE: u32 = 10_000_000;
const H264_MAGIC: &[u8; 8] = b"HEARTH26";
const H264_IDLE_KEEPALIVE: Duration = Duration::from_secs(1);
const H264_KEYFRAME_INTERVAL: Duration = Duration::from_secs(1);

pub(super) fn serve_mjpeg(stream: &mut TcpStream, state: &RemoteHostState) {
    let _capture_guard = match state.capture_guard.lock() {
        Ok(guard) => guard,
        Err(_) => {
            write_json_error(
                stream,
                500,
                "Internal Server Error",
                "화면 캡처 잠금 오류입니다.",
            );
            return;
        }
    };
    let mut capture = match desktop::ScreenCapture::new(MJPEG_WIDTH) {
        Ok(capture) => capture,
        Err(message) => {
            write_json_error(stream, 500, "Internal Server Error", &message);
            return;
        }
    };
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));

    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: multipart/x-mixed-replace; boundary={MJPEG_BOUNDARY}\r\nCache-Control: no-store, no-cache, must-revalidate\r\nPragma: no-cache\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n"
    );
    if stream.write_all(response.as_bytes()).is_err() {
        return;
    }

    let mut profile = MjpegProfile::default();
    let mut last_sent = Instant::now() - MJPEG_IDLE_REFRESH;
    let mut part_header = String::with_capacity(112);

    loop {
        let frame_started = Instant::now();
        if let Err(message) = capture.capture_frame(profile.width) {
            set_host_error(state, message);
            return;
        }

        if !capture.frame_changed() && last_sent.elapsed() < MJPEG_IDLE_REFRESH {
            sleep_until_next_frame(frame_started);
            continue;
        }

        let jpeg = match capture.encode_jpeg(profile.quality) {
            Ok(jpeg) => jpeg,
            Err(message) => {
                set_host_error(state, message);
                return;
            }
        };
        part_header.clear();
        let _ = write!(
            part_header,
            "--{MJPEG_BOUNDARY}\r\nContent-Type: image/jpeg\r\nContent-Length: {}\r\n\r\n",
            jpeg.len()
        );

        // Blocking writes provide backpressure: stale frames are never queued.
        if stream.write_all(part_header.as_bytes()).is_err()
            || stream.write_all(jpeg).is_err()
            || stream.write_all(b"\r\n").is_err()
        {
            return;
        }
        last_sent = Instant::now();

        let elapsed = frame_started.elapsed();
        profile.observe(elapsed);
        sleep_for_remainder(elapsed);
    }
}

pub(super) fn serve_h264(stream: &mut TcpStream, state: &RemoteHostState, tailscale: bool) {
    let _capture_guard = match state.capture_guard.lock() {
        Ok(guard) => guard,
        Err(_) => {
            write_json_error(
                stream,
                500,
                "Internal Server Error",
                "화면 캡처 잠금 오류입니다.",
            );
            return;
        }
    };

    let mut capture = match desktop::ScreenCapture::new(H264_WIDTH) {
        Ok(capture) => capture,
        Err(message) => {
            fail_h264_request(stream, state, 500, "Internal Server Error", message);
            return;
        }
    };
    if let Err(message) = capture.capture_frame(H264_WIDTH) {
        fail_h264_request(stream, state, 500, "Internal Server Error", message);
        return;
    }

    let (width, height) = capture.frame_dimensions();
    let bitrate = if tailscale {
        H264_TAILSCALE_BITRATE
    } else {
        H264_LAN_BITRATE
    };
    let mut encoder = match h264::H264Encoder::new(width, height, H264_FPS, bitrate) {
        Ok(encoder) => encoder,
        Err(message) => {
            fail_h264_request(stream, state, 503, "Service Unavailable", message);
            return;
        }
    };
    let encoder_name = encoder.encoder_name().to_owned();
    set_h264_ready(state, &encoder_name, encoder.is_hardware(), width, height);

    let _ = stream.set_write_timeout(Some(Duration::from_secs(3)));
    let response = "HTTP/1.1 200 OK\r\nContent-Type: application/x-heart-h264\r\nCache-Control: no-store\r\nAccess-Control-Allow-Origin: *\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n";
    if stream.write_all(response.as_bytes()).is_err()
        || write_h264_stream_header(
            stream,
            &H264StreamInfo {
                width,
                height,
                fps: H264_FPS,
                bitrate,
                hardware: encoder.is_hardware(),
                encoder_name: &encoder_name,
            },
        )
        .is_err()
    {
        return;
    }

    let stream_started = Instant::now();
    let mut last_keepalive = Instant::now();
    let mut last_keyframe = Instant::now() - H264_KEYFRAME_INTERVAL;

    loop {
        let frame_started = Instant::now();
        if let Err(message) = capture.capture_frame(H264_WIDTH) {
            set_h264_error(state, message);
            return;
        }
        if capture.frame_dimensions() != (width, height) {
            set_h264_error(
                state,
                "화면 크기가 변경되어 H.264 스트림을 다시 연결합니다.".to_string(),
            );
            return;
        }

        let units = if capture.frame_changed() {
            if last_keyframe.elapsed() >= H264_KEYFRAME_INTERVAL {
                encoder.force_keyframe();
                last_keyframe = Instant::now();
            }
            let timestamp_100ns =
                stream_started.elapsed().as_nanos().min(i64::MAX as u128) as i64 / 100;
            match encoder.encode(capture.nv12_frame(), timestamp_100ns) {
                Ok(units) => units,
                Err(message) => {
                    set_h264_error(state, message);
                    return;
                }
            }
        } else {
            match encoder.poll() {
                Ok(units) => units,
                Err(message) => {
                    set_h264_error(state, message);
                    return;
                }
            }
        };

        for unit in units {
            let flags = u8::from(unit.keyframe) | (u8::from(unit.has_configuration) << 1);
            if write_h264_packet(stream, unit.timestamp_us, flags, &unit.data).is_err() {
                return;
            }
            last_keepalive = Instant::now();
        }

        if last_keepalive.elapsed() >= H264_IDLE_KEEPALIVE {
            let timestamp_us = stream_started.elapsed().as_micros().min(u64::MAX as u128) as u64;
            if write_h264_packet(stream, timestamp_us, 0, &[]).is_err() {
                return;
            }
            last_keepalive = Instant::now();
        }
        sleep_until_next_frame(frame_started);
    }
}

#[derive(Debug)]
struct MjpegProfile {
    width: i32,
    quality: u8,
    pressure_frames: u8,
    recovery_frames: u16,
}

impl Default for MjpegProfile {
    fn default() -> Self {
        Self {
            width: MJPEG_WIDTH,
            quality: MJPEG_QUALITY,
            pressure_frames: 0,
            recovery_frames: 0,
        }
    }
}

impl MjpegProfile {
    fn observe(&mut self, elapsed: Duration) {
        if elapsed > MJPEG_SLOW_FRAME {
            self.pressure_frames = self.pressure_frames.saturating_add(1);
            self.recovery_frames = 0;
        } else {
            self.pressure_frames = 0;
            self.recovery_frames = self.recovery_frames.saturating_add(1);
        }

        if self.pressure_frames >= 5 {
            (self.width, self.quality) = lower_mjpeg_profile(self.width, self.quality);
            self.pressure_frames = 0;
        } else if self.recovery_frames >= 180 {
            (self.width, self.quality) = raise_mjpeg_profile(self.width, self.quality);
            self.recovery_frames = 0;
        }
    }
}

fn lower_mjpeg_profile(width: i32, quality: u8) -> (i32, u8) {
    match width {
        width if width > 960 => (960, quality.saturating_sub(4).max(42)),
        width if width > 840 => (840, quality.saturating_sub(4).max(42)),
        _ => (width, quality.saturating_sub(3).max(40)),
    }
}

fn raise_mjpeg_profile(width: i32, quality: u8) -> (i32, u8) {
    match width {
        width if width < 840 => (840, quality.saturating_add(3).min(MJPEG_QUALITY)),
        width if width < 960 => (960, quality.saturating_add(4).min(MJPEG_QUALITY)),
        width if width < MJPEG_WIDTH => (MJPEG_WIDTH, quality.saturating_add(4).min(MJPEG_QUALITY)),
        _ => (width, quality.saturating_add(2).min(MJPEG_QUALITY)),
    }
}

struct H264StreamInfo<'a> {
    width: u32,
    height: u32,
    fps: u32,
    bitrate: u32,
    hardware: bool,
    encoder_name: &'a str,
}

fn write_h264_stream_header<W: Write>(
    stream: &mut W,
    info: &H264StreamInfo<'_>,
) -> std::io::Result<()> {
    let name = info.encoder_name.as_bytes();
    let name_length = name.len().min(u16::MAX as usize) as u16;
    let mut header = Vec::with_capacity(28 + usize::from(name_length));
    header.extend_from_slice(H264_MAGIC);
    header.extend_from_slice(&1_u16.to_be_bytes());
    header.extend_from_slice(&u16::from(info.hardware).to_be_bytes());
    header.extend_from_slice(&(info.width as u16).to_be_bytes());
    header.extend_from_slice(&(info.height as u16).to_be_bytes());
    header.extend_from_slice(&(info.fps as u16).to_be_bytes());
    header.extend_from_slice(&0_u16.to_be_bytes());
    header.extend_from_slice(&info.bitrate.to_be_bytes());
    header.extend_from_slice(&name_length.to_be_bytes());
    header.extend_from_slice(&0_u16.to_be_bytes());
    header.extend_from_slice(&name[..usize::from(name_length)]);
    stream.write_all(&header)
}

fn write_h264_packet<W: Write>(
    stream: &mut W,
    timestamp_us: u64,
    flags: u8,
    payload: &[u8],
) -> std::io::Result<()> {
    let mut header = [0_u8; 16];
    header[..4].copy_from_slice(&(payload.len() as u32).to_be_bytes());
    header[4..12].copy_from_slice(&timestamp_us.to_be_bytes());
    header[12] = flags;
    stream.write_all(&header)?;
    stream.write_all(payload)
}

fn sleep_until_next_frame(started: Instant) {
    sleep_for_remainder(started.elapsed());
}

fn sleep_for_remainder(elapsed: Duration) {
    if let Some(remaining) = FRAME_INTERVAL.checked_sub(elapsed) {
        thread::sleep(remaining);
    }
}

fn set_host_error(state: &RemoteHostState, message: String) {
    if let Ok(mut inner) = state.inner.lock() {
        inner.error_message = Some(message);
    }
}

fn set_h264_ready(
    state: &RemoteHostState,
    encoder_name: &str,
    hardware: bool,
    width: u32,
    height: u32,
) {
    if let Ok(mut inner) = state.inner.lock() {
        inner.h264_encoder = Some(encoder_name.to_owned());
        inner.h264_hardware_accelerated = Some(hardware);
        inner.h264_error = None;
        inner.h264_width = Some(width);
        inner.h264_height = Some(height);
    }
}

fn set_h264_error(state: &RemoteHostState, message: String) {
    if let Ok(mut inner) = state.inner.lock() {
        inner.h264_error = Some(message);
    }
}

fn fail_h264_request(
    stream: &mut TcpStream,
    state: &RemoteHostState,
    status: u16,
    reason: &str,
    message: String,
) {
    set_h264_error(state, message.clone());
    write_json_error(stream, status, reason, &message);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn h264_wire_header_is_big_endian_and_self_delimiting() {
        let mut bytes = Vec::new();
        write_h264_stream_header(
            &mut bytes,
            &H264StreamInfo {
                width: 1_728,
                height: 1_080,
                fps: 60,
                bitrate: 18_000_000,
                hardware: true,
                encoder_name: "AMDh264Encoder",
            },
        )
        .unwrap();
        write_h264_packet(&mut bytes, 123_456, 0x03, &[0, 0, 0, 1, 0x65]).unwrap();

        assert_eq!(&bytes[..8], H264_MAGIC);
        assert_eq!(u16::from_be_bytes([bytes[8], bytes[9]]), 1);
        assert_eq!(u16::from_be_bytes([bytes[10], bytes[11]]), 1);
        assert_eq!(u16::from_be_bytes([bytes[12], bytes[13]]), 1_728);
        assert_eq!(u16::from_be_bytes([bytes[14], bytes[15]]), 1_080);
        assert_eq!(u16::from_be_bytes([bytes[16], bytes[17]]), 60);
        assert_eq!(
            u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]),
            18_000_000
        );
        let packet = 28 + usize::from(u16::from_be_bytes([bytes[24], bytes[25]]));
        assert_eq!(
            u32::from_be_bytes(bytes[packet..packet + 4].try_into().unwrap()),
            5
        );
        assert_eq!(bytes[packet + 12], 0x03);
        assert_eq!(&bytes[packet + 16..], &[0, 0, 0, 1, 0x65]);
    }

    #[test]
    fn mjpeg_profile_reacts_only_to_sustained_pressure() {
        let mut profile = MjpegProfile::default();
        for _ in 0..4 {
            profile.observe(Duration::from_millis(25));
        }
        assert_eq!(profile.width, MJPEG_WIDTH);
        profile.observe(Duration::from_millis(25));
        assert_eq!(profile.width, 960);

        for _ in 0..180 {
            profile.observe(Duration::from_millis(1));
        }
        assert_eq!(profile.width, MJPEG_WIDTH);
    }
}
