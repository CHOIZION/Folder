package org.heartlibrary.heartremote;

import android.content.Context;
import android.content.SharedPreferences;

final class ConnectionStore {
    private static final String PREFS_NAME = "heart_remote";
    private static final String PREF_URL = "heart_url";
    private static final String PREF_TOKEN = "heart_token";

    private final SharedPreferences preferences;

    ConnectionStore(Context context) {
        preferences = context.getSharedPreferences(PREFS_NAME, Context.MODE_PRIVATE);
    }

    SavedConnection load() {
        return new SavedConnection(
            preferences.getString(PREF_URL, ""),
            preferences.getString(PREF_TOKEN, "")
        );
    }

    void save(String baseUrl, String token) {
        preferences.edit().putString(PREF_URL, baseUrl).putString(PREF_TOKEN, token).apply();
    }

    void clear() {
        preferences.edit().remove(PREF_URL).remove(PREF_TOKEN).apply();
    }

    static final class SavedConnection {
        final String baseUrl;
        final String token;

        SavedConnection(String baseUrl, String token) {
            this.baseUrl = baseUrl;
            this.token = token;
        }

        boolean isComplete() {
            return !baseUrl.isEmpty() && !token.isEmpty();
        }
    }
}
