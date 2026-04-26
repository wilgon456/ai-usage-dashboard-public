package com.aiusagedashboard.widget;

import android.app.Activity;
import android.os.Bundle;
import android.view.Gravity;
import android.view.View;
import android.widget.Button;
import android.widget.EditText;
import android.widget.ImageView;
import android.widget.LinearLayout;
import android.widget.TextView;
import android.widget.Toast;

public class MainActivity extends Activity {
    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        UsageSnapshotStore store = new UsageSnapshotStore(this);

        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setGravity(Gravity.CENTER_HORIZONTAL);
        int pad = dp(24);
        root.setPadding(pad, pad, pad, pad);

        TextView title = new TextView(this);
        title.setText("AI Usage Dashboard");
        title.setTextSize(24);
        title.setGravity(Gravity.CENTER);
        title.setTextColor(0xff101827);
        root.addView(title, matchWrap());

        TextView status = new TextView(this);
        status.setText(store.hasSnapshot() ? "Widget data is ready." : "No widget data yet.");
        status.setTextSize(15);
        status.setGravity(Gravity.CENTER);
        status.setTextColor(0xff526071);
        LinearLayout.LayoutParams statusParams = matchWrap();
        statusParams.setMargins(0, dp(10), 0, dp(24));
        root.addView(status, statusParams);

        ImageView preview = new ImageView(this);
        preview.setAdjustViewBounds(true);
        preview.setScaleType(ImageView.ScaleType.FIT_CENTER);
        LinearLayout.LayoutParams previewParams = new LinearLayout.LayoutParams(dp(125), dp(125));
        previewParams.setMargins(0, 0, 0, dp(18));
        root.addView(preview, previewParams);
        updatePreview(preview, store);

        EditText syncUrl = new EditText(this);
        syncUrl.setSingleLine(true);
        syncUrl.setHint("https://relay.example.com/v1/snapshots/...?...token=...");
        syncUrl.setText(store.syncUrl());
        syncUrl.setTextSize(13);
        root.addView(syncUrl, matchWrap());

        Button sync = new Button(this);
        sync.setText("Sync Widget");
        sync.setAllCaps(false);
        LinearLayout.LayoutParams syncParams = matchWrap();
        syncParams.setMargins(0, dp(10), 0, dp(14));
        root.addView(sync, syncParams);

        Button seed = new Button(this);
        seed.setText("Seed Demo Snapshot");
        seed.setAllCaps(false);
        root.addView(seed, matchWrap());

        Button refresh = new Button(this);
        refresh.setText("Refresh Widgets");
        refresh.setAllCaps(false);
        LinearLayout.LayoutParams refreshParams = matchWrap();
        refreshParams.setMargins(0, dp(10), 0, 0);
        root.addView(refresh, refreshParams);

        seed.setOnClickListener((View view) -> {
            store.saveDemoSnapshot();
            updateWidgets();
            updatePreview(preview, store);
            status.setText("Demo widget data saved.");
            Toast.makeText(this, "Widget snapshot updated.", Toast.LENGTH_SHORT).show();
        });

        sync.setOnClickListener((View view) -> {
            String url = syncUrl.getText().toString().trim();
            if (url.isEmpty()) {
                Toast.makeText(this, "Enter the sync URL first.", Toast.LENGTH_SHORT).show();
                return;
            }
            store.saveSyncUrl(url);
            WidgetSyncWorker.ensurePeriodic(this);
            syncInBackground(store, preview, status, sync, "Widget synced.", true);
            WidgetPushRegistrar.registerSavedUrl(this);
        });

        refresh.setOnClickListener((View view) -> {
            updateWidgets();
            updatePreview(preview, store);
            Toast.makeText(this, "Widgets refreshed.", Toast.LENGTH_SHORT).show();
        });

        setContentView(root);
        updateWidgets();
        if (!store.syncUrl().trim().isEmpty()) {
            WidgetSyncWorker.ensurePeriodic(this);
            WidgetPushRegistrar.registerSavedUrl(this);
            syncInBackground(store, preview, status, sync, "Widget auto-synced.", false);
        } else {
            WidgetSyncWorker.cancelPeriodic(this);
        }
    }

    private void syncInBackground(
        UsageSnapshotStore store,
        ImageView preview,
        TextView status,
        Button syncButton,
        String successMessage,
        boolean showToast
    ) {
        status.setText("Syncing widget...");
        syncButton.setEnabled(false);
        new Thread(() -> {
            try {
                boolean synced = UsageSnapshotSync.syncSavedUrl(this);
                runOnUiThread(() -> {
                    syncButton.setEnabled(true);
                    updateWidgets();
                    updatePreview(preview, store);
                    if (synced) {
                        WidgetPushRegistrar.registerSavedUrl(this);
                    }
                    status.setText(synced ? successMessage : "No sync URL saved.");
                    if (showToast) {
                        Toast.makeText(this, synced ? successMessage : "No sync URL saved.", Toast.LENGTH_SHORT).show();
                    }
                });
            } catch (Exception error) {
                store.saveLastError(error.getMessage());
                runOnUiThread(() -> {
                    syncButton.setEnabled(true);
                    updateWidgets();
                    updatePreview(preview, store);
                    status.setText("Widget sync failed.");
                    if (showToast) {
                        Toast.makeText(this, store.safeErrorMessage(error.getMessage()), Toast.LENGTH_LONG).show();
                    }
                });
            }
        }).start();
    }

    private void updateWidgets() {
        UsageWidgetProvider.updateAllWidgets(this);
    }

    private void updatePreview(ImageView preview, UsageSnapshotStore store) {
        preview.setImageBitmap(WidgetTileRenderer.renderWidget(this, store.providers(), 125));
    }

    private LinearLayout.LayoutParams matchWrap() {
        return new LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        );
    }

    private int dp(int value) {
        return Math.round(value * getResources().getDisplayMetrics().density);
    }
}
