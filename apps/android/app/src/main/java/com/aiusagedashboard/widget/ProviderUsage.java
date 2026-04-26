package com.aiusagedashboard.widget;

final class ProviderUsage {
    final String id;
    final String name;
    final int percentUsed;
    final String usageLabel;
    final String summary;
    final int accentColor;

    ProviderUsage(String id, String name, int percentUsed, String usageLabel, String summary, int accentColor) {
        this.id = id;
        this.name = name;
        this.percentUsed = Math.max(0, Math.min(100, percentUsed));
        this.usageLabel = usageLabel;
        this.summary = summary;
        this.accentColor = accentColor;
    }
}
