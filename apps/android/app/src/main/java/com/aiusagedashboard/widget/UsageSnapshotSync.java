package com.aiusagedashboard.widget;

import android.content.Context;

import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.net.HttpURLConnection;
import java.net.URL;

final class UsageSnapshotSync {
    private static final int MAX_SNAPSHOT_BYTES = 128 * 1024;

    private UsageSnapshotSync() {}

    static boolean syncSavedUrl(Context context) throws Exception {
        UsageSnapshotStore store = new UsageSnapshotStore(context);
        String url = store.syncUrl().trim();
        if (url.isEmpty()) {
            return false;
        }

        String body = fetch(url);
        store.saveSnapshot(body);
        store.clearLastError();
        return true;
    }

    static String fetch(String rawUrl) throws Exception {
        HttpURLConnection connection = (HttpURLConnection) new URL(rawUrl).openConnection();
        connection.setConnectTimeout(5000);
        connection.setReadTimeout(5000);
        connection.setRequestMethod("GET");
        int status = connection.getResponseCode();
        if (status != 200) {
            throw new IllegalStateException("Sync returned HTTP " + status);
        }

        StringBuilder builder = new StringBuilder();
        int totalBytes = 0;
        try (BufferedReader reader = new BufferedReader(new InputStreamReader(connection.getInputStream()))) {
            String line;
            while ((line = reader.readLine()) != null) {
                totalBytes += line.getBytes(java.nio.charset.StandardCharsets.UTF_8).length;
                if (totalBytes > MAX_SNAPSHOT_BYTES) {
                    throw new IllegalStateException("Sync response too large");
                }
                builder.append(line);
            }
        } finally {
            connection.disconnect();
        }
        return builder.toString();
    }
}
