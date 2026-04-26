package com.aiusagedashboard.widget;

import android.content.Context;
import android.content.SharedPreferences;

import org.json.JSONArray;
import org.json.JSONException;
import org.json.JSONObject;

import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStreamReader;
import java.time.Instant;
import java.util.ArrayList;
import java.util.List;

final class UsageSnapshotStore {
    private static final String PREFS = "ai_usage_widget";
    private static final String KEY_SNAPSHOT = "snapshot_json";
    private static final String KEY_SYNC_URL = "sync_url";
    private static final String KEY_LAST_ERROR = "last_error";

    private final Context context;
    private final SharedPreferences prefs;

    UsageSnapshotStore(Context context) {
        this.context = context.getApplicationContext();
        prefs = context.getSharedPreferences(PREFS, Context.MODE_PRIVATE);
    }

    boolean hasSnapshot() {
        return prefs.contains(KEY_SNAPSHOT);
    }

    String syncUrl() {
        return prefs.getString(KEY_SYNC_URL, "");
    }

    void saveSyncUrl(String url) {
        prefs.edit().putString(KEY_SYNC_URL, url.trim()).apply();
    }

    void saveLastError(String message) {
        prefs.edit().putString(KEY_LAST_ERROR, redactErrorMessage(message, syncUrl())).apply();
    }

    String safeErrorMessage(String message) {
        return redactErrorMessage(message, syncUrl());
    }

    private String redactErrorMessage(String message, String syncUrl) {
        String safeMessage = message == null ? "Sync failed" : message;
        if (!syncUrl.isEmpty()) {
            safeMessage = safeMessage.replace(syncUrl, "[sync-url]");
        }
        return safeMessage.replaceAll("(?i)(token=)[^&\\s]+", "$1[redacted]");
    }

    void clearLastError() {
        prefs.edit().remove(KEY_LAST_ERROR).apply();
    }

    String lastError() {
        return prefs.getString(KEY_LAST_ERROR, "");
    }

    String fetchedAt() {
        try {
            JSONObject root = new JSONObject(snapshotJson());
            return root.optString("fetchedAt", "Not refreshed yet");
        } catch (JSONException ignored) {
            return "Not refreshed yet";
        }
    }

    List<ProviderUsage> providers() {
        ArrayList<ProviderUsage> items = new ArrayList<>();
        try {
            JSONObject root = new JSONObject(snapshotJson());
            JSONArray providers = root.optJSONArray("providers");
            if (providers == null) return items;

            for (int i = 0; i < providers.length(); i++) {
                JSONObject provider = providers.optJSONObject(i);
                if (provider == null) continue;
                items.add(new ProviderUsage(
                    provider.optString("id", ""),
                    provider.optString("name", "Provider"),
                    provider.optInt("percentUsed", 0),
                    provider.optString("usageLabel", compactUsageLabel(provider.optString("summary", ""))),
                    provider.optString("summary", ""),
                    parseColor(provider.optString("accentColor", "#667085"))
                ));
            }
        } catch (JSONException ignored) {
            return new ArrayList<>();
        }
        return items;
    }

    void saveDemoSnapshot() {
        prefs.edit().putString(KEY_SNAPSHOT, demoSnapshot()).apply();
    }

    void saveSnapshot(String snapshotJson) throws JSONException {
        JSONObject root = new JSONObject(snapshotJson);
        if (root.optJSONArray("providers") == null) {
            throw new JSONException("Missing providers array");
        }
        prefs.edit().putString(KEY_SNAPSHOT, root.toString()).apply();
    }

    private String snapshotJson() {
        String value = prefs.getString(KEY_SNAPSHOT, null);
        return value == null ? "{\"providers\":[]}" : value;
    }

    private String demoSnapshot() {
        JSONObject root = new JSONObject();
        JSONArray providers = new JSONArray();
        try {
            JSONObject registry = new JSONObject(readProviderRegistry());
            JSONArray order = registry.getJSONArray("defaultProviderOrder");
            JSONObject definitions = registry.getJSONObject("providers");

            for (int i = 0; i < order.length(); i++) {
                String id = order.getString(i);
                JSONObject definition = definitions.getJSONObject(id);
                JSONObject item = new JSONObject();
                item.put("id", id);
                item.put("name", definition.getString("displayName"));
                item.put("percentUsed", definition.optInt("demoPercentUsed", 0));
                item.put("summary", definition.optString("widgetSummary", ""));
                item.put("usageLabel", compactUsageLabel(definition.optString("widgetSummary", "")));
                item.put("accentColor", definition.optString("brandColor", "#667085"));
                providers.put(item);
            }

            root.put("fetchedAt", Instant.now().toString());
            root.put("providers", providers);
            return root.toString();
        } catch (JSONException | IOException ignored) {
            return "{\"providers\":[]}";
        }
    }

    private String readProviderRegistry() throws IOException {
        StringBuilder builder = new StringBuilder();
        try (BufferedReader reader = new BufferedReader(new InputStreamReader(
            context.getResources().openRawResource(R.raw.provider_registry)
        ))) {
            String line;
            while ((line = reader.readLine()) != null) {
                builder.append(line);
            }
        }
        return builder.toString();
    }

    private int parseColor(String value) {
        try {
            return android.graphics.Color.parseColor(value);
        } catch (IllegalArgumentException ignored) {
            return 0xff667085;
        }
    }

    private String compactUsageLabel(String value) {
        String trimmed = value.trim();
        int separator = trimmed.indexOf(":");
        if (separator >= 0 && separator + 1 < trimmed.length()) {
            trimmed = trimmed.substring(separator + 1).trim();
        }
        trimmed = trimmed
            .replaceAll("(?i)\\s+tokens?", " tok")
            .replaceAll("(?i)\\s+requests?", " req")
            .replaceAll("(?i)\\s+credits?", " cr");
        return trimmed.length() > 12 ? trimmed.substring(0, 12) : trimmed;
    }
}
