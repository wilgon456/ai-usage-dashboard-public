package com.aiusagedashboard.widget;

import com.google.firebase.messaging.FirebaseMessagingService;
import com.google.firebase.messaging.RemoteMessage;

public class UsageFirebaseMessagingService extends FirebaseMessagingService {
    @Override
    public void onNewToken(String token) {
        super.onNewToken(token);
        WidgetPushRegistrar.registerTokenInBackground(this, token);
    }

    @Override
    public void onMessageReceived(RemoteMessage message) {
        super.onMessageReceived(message);
        String type = message.getData().get("type");
        if (!"snapshot.updated".equals(type)) return;

        String pairId = message.getData().get("pairId");
        WidgetPushRegistrar.SyncTarget target = WidgetPushRegistrar.SyncTarget.fromUrl(
            new UsageSnapshotStore(this).syncUrl()
        );
        if (target == null || !target.pairId.equals(pairId)) return;

        WidgetSyncWorker.enqueueOneTime(this);
    }
}
