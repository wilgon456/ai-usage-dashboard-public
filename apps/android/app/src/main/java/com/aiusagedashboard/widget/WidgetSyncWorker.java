package com.aiusagedashboard.widget;

import android.content.Context;

import androidx.annotation.NonNull;
import androidx.work.Constraints;
import androidx.work.ExistingPeriodicWorkPolicy;
import androidx.work.ExistingWorkPolicy;
import androidx.work.NetworkType;
import androidx.work.OneTimeWorkRequest;
import androidx.work.PeriodicWorkRequest;
import androidx.work.WorkManager;
import androidx.work.Worker;
import androidx.work.WorkerParameters;

import java.util.concurrent.TimeUnit;

public class WidgetSyncWorker extends Worker {
    private static final String ONE_TIME_WORK = "ai_usage_widget_sync_once";
    private static final String PERIODIC_WORK = "ai_usage_widget_sync_periodic";

    public WidgetSyncWorker(@NonNull Context context, @NonNull WorkerParameters params) {
        super(context, params);
    }

    @NonNull
    @Override
    public Result doWork() {
        Context context = getApplicationContext();
        UsageSnapshotStore store = new UsageSnapshotStore(context);
        if (store.syncUrl().trim().isEmpty()) {
            return Result.success();
        }

        try {
            UsageSnapshotSync.syncSavedUrl(context);
            WidgetPushRegistrar.registerSavedUrl(context);
            UsageWidgetProvider.updateAllWidgets(context);
            return Result.success();
        } catch (Exception error) {
            store.saveLastError(error.getMessage());
            UsageWidgetProvider.updateAllWidgets(context);
            return Result.retry();
        }
    }

    static void enqueueOneTime(Context context) {
        if (new UsageSnapshotStore(context).syncUrl().trim().isEmpty()) {
            return;
        }
        OneTimeWorkRequest request = new OneTimeWorkRequest.Builder(WidgetSyncWorker.class)
            .setConstraints(networkConstraints())
            .build();
        WorkManager.getInstance(context.getApplicationContext()).enqueueUniqueWork(
            ONE_TIME_WORK,
            ExistingWorkPolicy.REPLACE,
            request
        );
    }

    static void ensurePeriodic(Context context) {
        if (new UsageSnapshotStore(context).syncUrl().trim().isEmpty()) {
            cancelPeriodic(context);
            return;
        }
        PeriodicWorkRequest request = new PeriodicWorkRequest.Builder(
            WidgetSyncWorker.class,
            15,
            TimeUnit.MINUTES
        )
            .setConstraints(networkConstraints())
            .build();
        WorkManager.getInstance(context.getApplicationContext()).enqueueUniquePeriodicWork(
            PERIODIC_WORK,
            ExistingPeriodicWorkPolicy.UPDATE,
            request
        );
    }

    static void cancelPeriodic(Context context) {
        WorkManager.getInstance(context.getApplicationContext()).cancelUniqueWork(PERIODIC_WORK);
    }

    private static Constraints networkConstraints() {
        return new Constraints.Builder()
            .setRequiredNetworkType(NetworkType.CONNECTED)
            .build();
    }
}
