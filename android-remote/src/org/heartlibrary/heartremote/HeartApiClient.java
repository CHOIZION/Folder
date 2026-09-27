package org.heartlibrary.heartremote;

import org.json.JSONObject;

import java.io.BufferedReader;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.charset.StandardCharsets;

final class HeartApiClient {
    static final int DEFAULT_PORT = 37218;
    private static final int TIMEOUT_MS = 4000;

    PairResult pair(String baseUrl, String code, String deviceName) throws Exception {
        JSONObject payload = new JSONObject();
        payload.put("code", code);
        payload.put("deviceName", deviceName);

        HttpResult response = request("POST", baseUrl + "/api/pair", null, payload.toString());
        JSONObject json = parseResponse(response, "페어링에 실패했습니다.");
        return new PairResult(json.getString("token"), json.optString("hostName", "HEART-PC"));
    }

    String heartbeat(String baseUrl, String token) throws Exception {
        HttpResult response = request("GET", baseUrl + "/api/heartbeat", token, null);
        return parseResponse(response, "HEART가 연결을 거부했습니다.")
            .optString("hostName", "HEART-PC");
    }

    String normalizeAddress(String input) {
        String value = input.trim();
        if (value.isEmpty()) throw new IllegalArgumentException("노트북 주소가 비어 있습니다.");
        if (!value.startsWith("http://") && !value.startsWith("https://")) {
            value = "http://" + value;
        }
        while (value.endsWith("/")) value = value.substring(0, value.length() - 1);

        final URL parsed;
        try {
            parsed = new URL(value);
        } catch (Exception error) {
            throw new IllegalArgumentException("예: 192.168.0.10 형식으로 입력해 주세요.");
        }
        if (parsed.getHost() == null || parsed.getHost().isEmpty()) {
            throw new IllegalArgumentException("노트북 IP 주소를 확인해 주세요.");
        }
        return parsed.getPort() < 0 ? value + ":" + DEFAULT_PORT : value;
    }

    private JSONObject parseResponse(HttpResult response, String fallbackMessage) throws Exception {
        JSONObject json = new JSONObject(response.body);
        if (response.status != HttpURLConnection.HTTP_OK) {
            throw new IllegalStateException(json.optString("message", fallbackMessage));
        }
        return json;
    }

    private HttpResult request(String method, String address, String token, String body) throws Exception {
        HttpURLConnection connection = (HttpURLConnection) new URL(address).openConnection();
        try {
            connection.setRequestMethod(method);
            connection.setConnectTimeout(TIMEOUT_MS);
            connection.setReadTimeout(TIMEOUT_MS);
            connection.setUseCaches(false);
            connection.setRequestProperty("Accept", "application/json");
            if (token != null && !token.isEmpty()) {
                connection.setRequestProperty("Authorization", "Bearer " + token);
            }
            if (body != null) writeJsonBody(connection, body);

            int status = connection.getResponseCode();
            InputStream stream = status >= 400 ? connection.getErrorStream() : connection.getInputStream();
            return new HttpResult(status, readFully(stream));
        } finally {
            connection.disconnect();
        }
    }

    private void writeJsonBody(HttpURLConnection connection, String body) throws Exception {
        byte[] bytes = body.getBytes(StandardCharsets.UTF_8);
        connection.setDoOutput(true);
        connection.setRequestProperty("Content-Type", "application/json; charset=utf-8");
        connection.setFixedLengthStreamingMode(bytes.length);
        try (OutputStream output = connection.getOutputStream()) {
            output.write(bytes);
        }
    }

    private String readFully(InputStream stream) throws Exception {
        if (stream == null) return "";
        StringBuilder result = new StringBuilder();
        try (BufferedReader reader = new BufferedReader(
            new InputStreamReader(stream, StandardCharsets.UTF_8)
        )) {
            String line;
            while ((line = reader.readLine()) != null) result.append(line);
        }
        return result.toString();
    }

    static final class PairResult {
        final String token;
        final String hostName;

        PairResult(String token, String hostName) {
            this.token = token;
            this.hostName = hostName;
        }
    }

    private static final class HttpResult {
        final int status;
        final String body;

        HttpResult(int status, String body) {
            this.status = status;
            this.body = body;
        }
    }
}
