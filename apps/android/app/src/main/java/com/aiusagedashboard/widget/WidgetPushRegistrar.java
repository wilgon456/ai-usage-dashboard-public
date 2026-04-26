package com.aiusagedashboard.widget;

import android.content.Context;
import android.net.Uri;

import com.google.firebase.FirebaseApp;
import com.google.firebase.messaging.FirebaseMessaging;

import org.json.JSONObject;

import java.io.OutputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.nio.charset.StandardCharsets;
import java.util.UUID;

final class WidgetPushRegistrar {
    private static final String APP_VERSION = "0.1.0";
    private static final String KEY_DEVICE_ID = "push_device_id";

    private WidgetPushRegistrar() {}

    static void registerSavedUrl(Context context) {
        UsageSnapshotStore store = new UsageSnapshotStore(context);
        SyncTarget target = SyncTarget.fromUrl(store.syncUrl());
        if (target == null) return;

        try {
            ensureFirebaseInitialized(context);
            FirebaseMessaging.getInstance().getToken()
                .addOnSuccessListener((String token) -> registerTokenInBackground(context, target, token))
                .addOnFailureListener((Exception ignored) -> {
                    // Push is best-effort; WorkManager polling remains the fallback.
                });
        } catch (IllegalStateException ignored) {
            // google-services configuration may be absent in local/debug builds.
        }
    }

    static void registerTokenInBackground(Context context, String pushToken) {
        SyncTarget target = SyncTarget.fromUrl(new UsageSnapshotStore(context).syncUrl());
        if (target == null || pushToken == null || pushToken.trim().isEmpty()) return;
        registerTokenInBackground(context, target, pushToken.trim());
    }

    private static void registerTokenInBackground(Context context, SyncTarget target, String pushToken) {
        new Thread(() -> {
            try {
                postRegistration(context.getApplicationContext(), target, pushToken);
            } catch (Exception ignored) {
                // Registration failure must not break snapshot polling or widget rendering.
            }
        }).start();
    }

    private static void postRegistration(Context context, SyncTarget target, String pushToken) throws Exception {
        URL url = new URL(target.registerUrl());
        HttpURLConnection connection = (HttpURLConnection) url.openConnection();
        connection.setRequestMethod("POST");
        connection.setConnectTimeout(8000);
        connection.setReadTimeout(8000);
        connection.setDoOutput(true);
        connection.setRequestProperty("Authorization", "Bearer " + target.token);
        connection.setRequestProperty("Content-Type", "application/json");

        JSONObject body = new JSONObject();
        body.put("platform", "android");
        body.put("provider", "fcm");
        body.put("pushToken", pushToken);
        body.put("appVersion", APP_VERSION);
        body.put("deviceId", deviceId(context));

        byte[] bytes = body.toString().getBytes(StandardCharsets.UTF_8);
        try (OutputStream stream = connection.getOutputStream()) {
            stream.write(bytes);
        }

        int status = connection.getResponseCode();
        connection.disconnect();
        if (status < 200 || status >= 300) {
            throw new IllegalStateException("Push registration failed: HTTP " + status);
        }
    }

    private static String deviceId(Context context) {
        UsageSnapshotStore store = new UsageSnapshotStore(context);
        android.content.SharedPreferences prefs = context.getSharedPreferences("ai_usage_widget", Context.MODE_PRIVATE);
        String id = prefs.getString(KEY_DEVICE_ID, "");
        if (id == null || id.isEmpty()) {
            id = UUID.randomUUID().toString();
            prefs.edit().putString(KEY_DEVICE_ID, id).apply();
        }
        return id;
    }

    private static void ensureFirebaseInitialized(Context context) {
        if (FirebaseApp.getApps(context).isEmpty()) {
            FirebaseApp.initializeApp(context);
        }
    }

    static final class SyncTarget {
        final String base;
        final String pairId;
        final String token;

        SyncTarget(String base, String pairId, String token) {
            this.base = base;
            this.pairId = pairId;
            this.token = token;
        }

        static SyncTarget fromUrl(String syncUrl) {
            if (syncUrl == null || syncUrl.trim().isEmpty()) return null;
            try {
                Uri uri = Uri.parse(syncUrl.trim());
                String path = uri.getPath();
                String token = uri.getQueryParameter("token");
                if (path == null || token == null || token.isEmpty()) return null;
                String marker = "/v1/snapshots/";
                int index = path.indexOf(marker);
                if (index < 0) return null;
                String pairId = path.substring(index + marker.length());
                if (pairId.isEmpty() || pairId.contains("/")) return null;
                String basePath = path.substring(0, index);
                String base = uri.getScheme() + "://" + uri.getEncodedAuthority() + basePath;
                return new SyncTarget(base, pairId, token);
            } catch (Exception ignored) {
                return null;
            }
        }

        String registerUrl() {
            return base + "/v1/push/" + pairId + "/register";
        }
    }
}
