package org.heartlibrary.heartremote;

import android.media.MediaCodec;
import android.media.MediaFormat;
import android.os.Build;
import android.os.SystemClock;
import android.view.Surface;

import java.io.ByteArrayOutputStream;
import java.io.DataInputStream;
import java.io.InputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.net.URLEncoder;
import java.nio.ByteBuffer;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;

final class H264StreamClient {
    interface Listener {
        void onStreamInfo(StreamInfo info);
        void onStats(StreamStats stats);
        void onRecovering(String message, long retryDelayMs);
        void onFatalError(String message);
    }

    static final class StreamInfo {
        final int width;
        final int height;
        final int fps;
        final int bitrate;
        final boolean hardware;
        final String encoderName;

        StreamInfo(int width, int height, int fps, int bitrate, boolean hardware, String encoderName) {
            this.width = width;
            this.height = height;
            this.fps = fps;
            this.bitrate = bitrate;
            this.hardware = hardware;
            this.encoderName = encoderName;
        }
    }

    static final class StreamStats {
        final int receivedFps;
        final int displayedFps;
        final long estimatedLatencyMs;

        StreamStats(int receivedFps, int displayedFps, long estimatedLatencyMs) {
            this.receivedFps = receivedFps;
            this.displayedFps = displayedFps;
            this.estimatedLatencyMs = estimatedLatencyMs;
        }
    }

    private static final byte[] MAGIC = "HEARTH26".getBytes(StandardCharsets.US_ASCII);
    private static final int MAX_PACKET_BYTES = 8 * 1024 * 1024;
    private static final int FLAG_KEYFRAME = 0x01;
    private static final int FLAG_CONFIGURATION = 0x02;

    private final String streamUrl;
    private final Surface surface;
    private final Listener listener;
    private Thread worker;
    private volatile boolean running;
    private volatile HttpURLConnection connection;
    private MediaCodec decoder;

    H264StreamClient(String baseUrl, String token, Surface surface, Listener listener) {
        this.streamUrl = buildStreamUrl(baseUrl, token);
        this.surface = surface;
        this.listener = listener;
    }

    void start() {
        if (running) return;
        running = true;
        worker = new Thread(this::run, "heart-h264-stream");
        worker.setDaemon(true);
        worker.start();
    }

    void stop() {
        running = false;
        HttpURLConnection active = connection;
        if (active != null) active.disconnect();
        Thread activeWorker = worker;
        if (activeWorker != null) activeWorker.interrupt();
    }

    private void run() {
        int failures = 0;
        try {
            while (running) {
                try {
                    streamOnce();
                    if (running) throw new IllegalStateException("H.264 스트림이 종료되었습니다.");
                } catch (FatalStreamException error) {
                    if (running) listener.onFatalError(error.getMessage());
                    return;
                } catch (Exception error) {
                    if (!running) return;
                    releaseDecoder();
                    failures += 1;
                    long delay = Math.min(3000L, 350L << Math.min(failures - 1, 3));
                    listener.onRecovering(ErrorMessage.readable(error), delay);
                    try {
                        Thread.sleep(delay);
                    } catch (InterruptedException interrupted) {
                        Thread.currentThread().interrupt();
                        return;
                    }
                }
            }
        } finally {
            releaseDecoder();
            connection = null;
        }
    }

    private void streamOnce() throws Exception {
        HttpURLConnection active = (HttpURLConnection) new URL(streamUrl).openConnection();
        connection = active;
        active.setRequestMethod("GET");
        active.setConnectTimeout(8000);
        active.setReadTimeout(0);
        active.setUseCaches(false);
        active.setRequestProperty("Accept", "application/x-heart-h264");
        int status = active.getResponseCode();
        if (status != HttpURLConnection.HTTP_OK) {
            String message = readError(active.getErrorStream());
            if (status == 401 || status == 404 || status == 503) {
                throw new FatalStreamException(message.isEmpty() ? "H.264 스트림을 사용할 수 없습니다." : message);
            }
            throw new IllegalStateException(message.isEmpty() ? "H.264 연결 오류: HTTP " + status : message);
        }

        try (DataInputStream input = new DataInputStream(active.getInputStream())) {
            StreamInfo info = readHeader(input);
            configureDecoder(info);
            listener.onStreamInfo(info);
            readPackets(input);
        } finally {
            active.disconnect();
            if (connection == active) connection = null;
            releaseDecoder();
        }
    }

    private StreamInfo readHeader(DataInputStream input) throws Exception {
        byte[] magic = new byte[MAGIC.length];
        input.readFully(magic);
        if (!Arrays.equals(magic, MAGIC)) throw new FatalStreamException("H.264 스트림 헤더가 올바르지 않습니다.");
        int version = input.readUnsignedShort();
        if (version != 1) throw new FatalStreamException("지원하지 않는 H.264 스트림 버전입니다: " + version);
        boolean hardware = (input.readUnsignedShort() & 1) != 0;
        int width = input.readUnsignedShort();
        int height = input.readUnsignedShort();
        int fps = input.readUnsignedShort();
        input.readUnsignedShort();
        int bitrate = input.readInt();
        int nameLength = input.readUnsignedShort();
        input.readUnsignedShort();
        if (width <= 0 || height <= 0 || nameLength > 4096) {
            throw new FatalStreamException("H.264 스트림 정보가 올바르지 않습니다.");
        }
        byte[] name = new byte[nameLength];
        input.readFully(name);
        return new StreamInfo(
            width,
            height,
            fps,
            bitrate,
            hardware,
            new String(name, StandardCharsets.UTF_8)
        );
    }

    private void configureDecoder(StreamInfo info) throws Exception {
        releaseDecoder();
        MediaFormat format = MediaFormat.createVideoFormat(MediaFormat.MIMETYPE_VIDEO_AVC, info.width, info.height);
        format.setInteger(MediaFormat.KEY_MAX_INPUT_SIZE, MAX_PACKET_BYTES);
        format.setInteger(MediaFormat.KEY_FRAME_RATE, info.fps);
        format.setInteger(MediaFormat.KEY_PRIORITY, 0);
        if (Build.VERSION.SDK_INT >= 30) format.setInteger(MediaFormat.KEY_LOW_LATENCY, 1);

        decoder = MediaCodec.createDecoderByType(MediaFormat.MIMETYPE_VIDEO_AVC);
        decoder.configure(format, surface, null, 0);
        decoder.start();
        decoder.setVideoScalingMode(MediaCodec.VIDEO_SCALING_MODE_SCALE_TO_FIT);
    }

    private void readPackets(DataInputStream input) throws Exception {
        MediaCodec.BufferInfo bufferInfo = new MediaCodec.BufferInfo();
        int received = 0;
        int displayed = 0;
        long statsStartedNs = SystemClock.elapsedRealtimeNanos();
        long streamClockBaseUs = Long.MIN_VALUE;
        long latestLatencyMs = 0;
        byte[] packetBuffer = new byte[0];

        while (running) {
            int length = input.readInt();
            long timestampUs = input.readLong();
            int flags = input.readUnsignedByte();
            input.skipBytes(3);
            if (length < 0 || length > MAX_PACKET_BYTES) {
                throw new IllegalStateException("H.264 패킷 크기가 허용 범위를 벗어났습니다: " + length);
            }
            if (length == 0) {
                displayed += drainOutput(bufferInfo);
            } else {
                packetBuffer = ensureCapacity(packetBuffer, length);
                input.readFully(packetBuffer, 0, length);
                if (streamClockBaseUs == Long.MIN_VALUE) {
                    streamClockBaseUs = elapsedUs() - timestampUs;
                }
                displayed += queueInput(packetBuffer, length, timestampUs, flags, bufferInfo);
                received += 1;
                displayed += drainOutput(bufferInfo);
                if (bufferInfo.presentationTimeUs > 0 && streamClockBaseUs != Long.MIN_VALUE) {
                    latestLatencyMs = Math.max(
                        0,
                        (elapsedUs() - (streamClockBaseUs + bufferInfo.presentationTimeUs)) / 1000
                    );
                }
            }

            long now = SystemClock.elapsedRealtimeNanos();
            if (now - statsStartedNs >= 1_000_000_000L) {
                listener.onStats(new StreamStats(received, displayed, latestLatencyMs));
                received = 0;
                displayed = 0;
                statsStartedNs = now;
            }
        }
    }

    private int queueInput(
        byte[] payload,
        int length,
        long timestampUs,
        int packetFlags,
        MediaCodec.BufferInfo bufferInfo
    ) throws Exception {
        int attempts = 0;
        int displayed = 0;
        while (running && attempts++ < 12) {
            int index = decoder.dequeueInputBuffer(10_000);
            if (index < 0) {
                displayed += drainOutput(bufferInfo);
                continue;
            }
            ByteBuffer buffer = decoder.getInputBuffer(index);
            if (buffer == null || buffer.capacity() < length) {
                throw new IllegalStateException("MediaCodec 입력 버퍼가 H.264 프레임보다 작습니다.");
            }
            buffer.clear();
            buffer.put(payload, 0, length);
            int codecFlags = 0;
            boolean keyframe = (packetFlags & FLAG_KEYFRAME) != 0;
            if (!keyframe && (packetFlags & FLAG_CONFIGURATION) != 0) {
                codecFlags |= MediaCodec.BUFFER_FLAG_CODEC_CONFIG;
            }
            decoder.queueInputBuffer(index, 0, length, timestampUs, codecFlags);
            return displayed;
        }
        throw new IllegalStateException("MediaCodec 입력 버퍼를 제시간에 확보하지 못했습니다.");
    }

    private static byte[] ensureCapacity(byte[] buffer, int required) {
        if (buffer.length >= required) return buffer;
        int capacity = Math.max(64 * 1024, buffer.length);
        while (capacity < required) {
            capacity = Math.min(MAX_PACKET_BYTES, capacity * 2);
        }
        return new byte[capacity];
    }

    private int drainOutput(MediaCodec.BufferInfo info) {
        if (decoder == null) return 0;
        int displayed = 0;
        while (running) {
            int index = decoder.dequeueOutputBuffer(info, 0);
            if (index >= 0) {
                boolean render = (info.flags & MediaCodec.BUFFER_FLAG_CODEC_CONFIG) == 0;
                decoder.releaseOutputBuffer(index, render);
                if (render) displayed += 1;
                continue;
            }
            if (index == MediaCodec.INFO_OUTPUT_FORMAT_CHANGED) continue;
            break;
        }
        return displayed;
    }

    private void releaseDecoder() {
        MediaCodec active = decoder;
        decoder = null;
        if (active == null) return;
        try {
            active.stop();
        } catch (Exception ignored) { }
        active.release();
    }

    private static String buildStreamUrl(String baseUrl, String token) {
        try {
            String host = new URL(baseUrl).getHost().toLowerCase();
            boolean tailscale = host.startsWith("100.") || host.endsWith(".ts.net");
            return baseUrl
                + "/api/screen/h264?token="
                + URLEncoder.encode(token, "UTF-8")
                + "&network="
                + (tailscale ? "tailscale" : "lan");
        } catch (Exception error) {
            return baseUrl + "/api/screen/h264?token=" + token + "&network=lan";
        }
    }

    private static String readError(InputStream stream) {
        if (stream == null) return "";
        try (InputStream input = stream; ByteArrayOutputStream output = new ByteArrayOutputStream()) {
            byte[] buffer = new byte[1024];
            int length;
            while ((length = input.read(buffer)) >= 0 && output.size() < 16 * 1024) {
                output.write(buffer, 0, length);
            }
            String body = output.toString("UTF-8");
            int message = body.indexOf("\"message\":\"");
            if (message >= 0) {
                int start = message + 11;
                int end = body.indexOf('"', start);
                if (end > start) return body.substring(start, end);
            }
            return body;
        } catch (Exception ignored) {
            return "";
        }
    }

    private static long elapsedUs() {
        return SystemClock.elapsedRealtimeNanos() / 1000L;
    }

    private static final class FatalStreamException extends Exception {
        private static final long serialVersionUID = 1L;

        FatalStreamException(String message) {
            super(message);
        }
    }
}
