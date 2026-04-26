package com.aiusagedashboard.widget;

import android.app.PendingIntent;
import android.appwidget.AppWidgetManager;
import android.appwidget.AppWidgetProvider;
import android.content.ComponentName;
import android.content.Context;
import android.content.Intent;
import android.view.View;
import android.widget.RemoteViews;

import java.util.List;

public class UsageWidgetProvider extends AppWidgetProvider {
    @Override
    public void onReceive(Context context, Intent intent) {
        super.onReceive(context, intent);
        if (Intent.ACTION_MY_PACKAGE_REPLACED.equals(intent.getAction())) {
            WidgetSyncWorker.ensurePeriodic(context);
            WidgetSyncWorker.enqueueOneTime(context);
            updateAllWidgets(context);
        }
    }

    @Override
    public void onUpdate(Context context, AppWidgetManager appWidgetManager, int[] appWidgetIds) {
        UsageSnapshotStore store = new UsageSnapshotStore(context);
        for (int appWidgetId : appWidgetIds) {
            appWidgetManager.updateAppWidget(appWidgetId, buildViews(context, store));
        }
        WidgetSyncWorker.ensurePeriodic(context);
        WidgetSyncWorker.enqueueOneTime(context);
    }

    private static RemoteViews buildViews(Context context, UsageSnapshotStore store) {
        RemoteViews views = new RemoteViews(context.getPackageName(), R.layout.usage_widget);
        List<ProviderUsage> providers = store.providers();

        views.setImageViewBitmap(R.id.widget_image, WidgetTileRenderer.renderWidget(context, providers, 125));
        views.setViewVisibility(R.id.empty_state, providers.isEmpty() ? View.VISIBLE : View.GONE);
        views.setViewVisibility(R.id.widget_image, providers.isEmpty() ? View.GONE : View.VISIBLE);

        Intent intent = new Intent(context, MainActivity.class);
        PendingIntent pendingIntent = PendingIntent.getActivity(
            context,
            0,
            intent,
            PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE
        );
        views.setOnClickPendingIntent(R.id.widget_root, pendingIntent);
        return views;
    }

    static void updateAllWidgets(Context context) {
        AppWidgetManager manager = AppWidgetManager.getInstance(context);
        int[] ids = manager.getAppWidgetIds(new ComponentName(context, UsageWidgetProvider.class));
        UsageSnapshotStore store = new UsageSnapshotStore(context);
        for (int appWidgetId : ids) {
            manager.updateAppWidget(appWidgetId, buildViews(context, store));
        }
    }
}
