package org.heartlibrary.heartremote;

import org.json.JSONObject;

import java.io.InputStream;
import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.LinkedBlockingQueue;
import java.util.concurrent.Semaphore;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicInteger;

final class RemoteInputDispatcher {
    private static final int TIMEOUT_MS = 1800;

    private final URL endpoint;
    private final String authorization;
    private final LinkedBlockingQueue<JSONObject> controls = new LinkedBlockingQueue<>();
    private final AtomicInteger verticalWheel = new AtomicInteger();
    private final AtomicInteger horizontalWheel = new AtomicInteger();
    private final Semaphore wakeup = new Semaphore(0);
    private final Thread worker;
    private final byte[] responseBuffer = new byte[256];
    private double latestMoveX;
    private double latestMoveY;
    private boolean movePending;
    private volatile boolean running = true;

    RemoteInputDispatcher(String baseUrl, String token) {
        try {
            this.endpoint = new URL(baseUrl + "/api/input");
        } catch (Exception error) {
            throw new IllegalArgumentException("원격 입력 주소가 올바르지 않습니다.", error);
        }
        this.authorization = "Bearer " + token;
        worker = new Thread(this::run, "heart-remote-input");
        worker.setDaemon(true);
        worker.start();
    }

    synchronized void move(double x, double y) {
        latestMoveX = clamp(x, 0.0, 1.0);
        latestMoveY = clamp(y, 0.0, 1.0);
        movePending = true;
        wakeup.release();
    }

    void button(String action, double x, double y) {
        JSONObject command = pointer(action, x, y);
        put(command, "button", "left");
        controls.offer(command);
        wakeup.release();
    }

    void key(String key) {
        JSONObject command = action("key");
        put(command, "key", key);
        controls.offer(command);
        wakeup.release();
    }

    void text(String value) {
        JSONObject command = action("text");
        put(command, "text", value);
        controls.offer(command);
        wakeup.release();
    }

    void wheel(int delta, boolean horizontal) {
        AtomicInteger target = horizontal ? horizontalWheel : verticalWheel;
        target.updateAndGet(current -> clamp(current + delta, -1200, 1200));
        wakeup.release();
    }

    void close() {
        running = false;
        controls.clear();
        synchronized (this) {
            movePending = false;
        }
        wakeup.release();
        worker.interrupt();
    }

    private void run() {
        while (running) {
            try {
                JSONObject command = controls.poll();
                if (command == null) command = nextCoalescedCommand();
                if (command != null) {
                    send(command);
                    continue;
                }
                wakeup.tryAcquire(8, TimeUnit.MILLISECONDS);
                wakeup.drainPermits();
            } catch (InterruptedException ignored) {
                Thread.currentThread().interrupt();
                return;
            } catch (Exception ignored) {
                // A failed move is obsolete immediately. Control commands are
                // deliberately not retried to avoid duplicate clicks or keys.
            }
        }
    }

    private synchronized JSONObject takeLatestMove() {
        if (!movePending) return null;
        movePending = false;
        return pointer("move", latestMoveX, latestMoveY);
    }

    private JSONObject nextCoalescedCommand() {
        JSONObject move = takeLatestMove();
        if (move != null) return move;

        int vertical = verticalWheel.getAndSet(0);
        if (vertical != 0) return wheelCommand("wheel", vertical);
        int horizontal = horizontalWheel.getAndSet(0);
        if (horizontal != 0) return wheelCommand("horizontalWheel", horizontal);
        return null;
    }

    private JSONObject wheelCommand(String action, int delta) {
        JSONObject command = action(action);
        put(command, "delta", delta);
        return command;
    }

    private JSONObject pointer(String action, double x, double y) {
        JSONObject command = action(action);
        put(command, "x", clamp(x, 0.0, 1.0));
        put(command, "y", clamp(y, 0.0, 1.0));
        return command;
    }

    private JSONObject action(String action) {
        JSONObject command = new JSONObject();
        put(command, "action", action);
        return command;
    }

    private void send(JSONObject command) throws Exception {
        byte[] body = command.toString().getBytes(StandardCharsets.UTF_8);
        HttpURLConnection connection = (HttpURLConnection) endpoint.openConnection();
        try {
            connection.setRequestMethod("POST");
            connection.setConnectTimeout(TIMEOUT_MS);
            connection.setReadTimeout(TIMEOUT_MS);
            connection.setUseCaches(false);
            connection.setDoOutput(true);
            connection.setRequestProperty("Authorization", authorization);
            connection.setRequestProperty("Content-Type", "application/json; charset=utf-8");
            connection.setFixedLengthStreamingMode(body.length);
            try (OutputStream output = connection.getOutputStream()) {
                output.write(body);
            }
            InputStream response = connection.getResponseCode() >= 400
                ? connection.getErrorStream()
                : connection.getInputStream();
            if (response != null) {
                try (InputStream ignored = response) {
                    while (ignored.read(responseBuffer) >= 0) { }
                }
            }
        } finally {
            connection.disconnect();
        }
    }

    private static void put(JSONObject target, String key, Object value) {
        try {
            target.put(key, value);
        } catch (Exception ignored) { }
    }

    private static int clamp(int value, int minimum, int maximum) {
        return Math.max(minimum, Math.min(maximum, value));
    }

    private static double clamp(double value, double minimum, double maximum) {
        return Math.max(minimum, Math.min(maximum, value));
    }
}
