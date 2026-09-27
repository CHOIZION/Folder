package org.heartlibrary.heartremote;

import java.util.Locale;

final class ErrorMessage {
    private ErrorMessage() {}

    static String readable(Exception error) {
        String message = error.getMessage();
        if (message == null || message.trim().isEmpty()) {
            message = error.getClass().getSimpleName();
        }
        String lower = message.toLowerCase(Locale.ROOT);
        if (lower.contains("failed to connect") || lower.contains("connection refused")) {
            return "노트북 HEART가 실행 중인지, 방화벽에서 개인 네트워크가 허용됐는지 확인하세요.";
        }
        if (lower.contains("timed out") || lower.contains("timeout")) {
            return "응답 시간이 초과되었습니다. 두 기기가 같은 Wi-Fi인지 확인하세요.";
        }
        return message;
    }
}
