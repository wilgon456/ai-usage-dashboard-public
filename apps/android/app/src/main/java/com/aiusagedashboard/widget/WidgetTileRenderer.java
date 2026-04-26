package com.aiusagedashboard.widget;

import android.content.Context;
import android.graphics.Bitmap;
import android.graphics.Canvas;
import android.graphics.Color;
import android.graphics.Paint;
import android.graphics.Rect;
import android.graphics.RectF;
import android.graphics.Typeface;
import android.graphics.drawable.Drawable;

final class WidgetTileRenderer {
    private WidgetTileRenderer() {}

    static Bitmap renderWidget(Context context, java.util.List<ProviderUsage> providers, int sizeDp) {
        float density = context.getResources().getDisplayMetrics().density;
        int size = Math.round(sizeDp * density);
        float scale = size / 125f;

        Bitmap bitmap = Bitmap.createBitmap(size, size, Bitmap.Config.ARGB_8888);
        Canvas canvas = new Canvas(bitmap);

        Paint paint = new Paint(Paint.ANTI_ALIAS_FLAG);
        paint.setStyle(Paint.Style.FILL);
        paint.setColor(Color.rgb(27, 28, 37));
        canvas.drawRoundRect(new RectF(0, 0, size, size), 23f * scale, 23f * scale, paint);

        float left = 3f * scale;
        float top = 3f * scale;
        float[][] positions = {
            { left, top },
            { 60f * scale, top },
            { left, 60f * scale },
            { 60f * scale, 60f * scale }
        };

        for (int i = 0; i < Math.min(4, providers.size()); i++) {
            drawGauge(context, canvas, providers.get(i), positions[i][0], positions[i][1], 58f * scale);
        }

        return bitmap;
    }

    static Bitmap render(Context context, ProviderUsage provider, int sizeDp) {
        float density = context.getResources().getDisplayMetrics().density;
        int size = Math.round(sizeDp * density);
        Bitmap bitmap = Bitmap.createBitmap(size, size, Bitmap.Config.ARGB_8888);
        Canvas canvas = new Canvas(bitmap);
        drawGauge(context, canvas, provider, 0, 0, size);
        return bitmap;
    }

    private static void drawGauge(
        Context context,
        Canvas canvas,
        ProviderUsage provider,
        float x,
        float y,
        float size
    ) {
        float scale = size / 58f;

        Paint paint = new Paint(Paint.ANTI_ALIAS_FLAG);
        float stroke = 5.5f * scale;
        float radius = 22f * scale;
        float centerX = x + 31f * scale;
        float centerY = y + 27f * scale;
        RectF ring = new RectF(
            centerX - radius,
            centerY - radius,
            centerX + radius,
            centerY + radius
        );

        paint.setStyle(Paint.Style.STROKE);
        paint.setStrokeCap(Paint.Cap.ROUND);
        paint.setStrokeWidth(stroke);
        paint.setColor(Color.rgb(59, 62, 78));
        float startAngle = -225f;
        float sweepAngle = 270f;
        canvas.drawArc(ring, startAngle, sweepAngle, false, paint);

        paint.setColor(gaugeColor(provider.percentUsed));
        canvas.drawArc(ring, startAngle, provider.percentUsed * (sweepAngle / 100f), false, paint);

        drawLogo(context, canvas, provider, x, y, size, scale);

        paint.setStyle(Paint.Style.FILL);
        paint.setStrokeWidth(0f);
        paint.setTypeface(Typeface.create(Typeface.DEFAULT, Typeface.NORMAL));
        paint.setTextAlign(Paint.Align.CENTER);
        paint.setColor(Color.rgb(238, 240, 247));
        paint.setTypeface(Typeface.create(Typeface.DEFAULT, Typeface.BOLD));
        paint.setTextSize(9.5f * scale);
        drawCenteredText(
            canvas,
            provider.percentUsed + "%",
            x + size / 2f,
            y + size - 11.5f * scale,
            paint
        );

        String usageLabel = provider.usageLabel.trim();
        if (!usageLabel.isEmpty()) {
            paint.setTypeface(Typeface.create(Typeface.DEFAULT, Typeface.BOLD));
            paint.setTextSize(6f * scale);
            paint.setColor(Color.rgb(183, 188, 204));
            drawCenteredText(
                canvas,
                usageLabel,
                x + size / 2f,
                y + size - 3.7f * scale,
                paint
            );
        }
    }

    private static void drawCenteredText(Canvas canvas, String text, float x, float centerY, Paint paint) {
        Rect bounds = new Rect();
        paint.getTextBounds(text, 0, text.length(), bounds);
        canvas.drawText(text, x, centerY - bounds.exactCenterY(), paint);
    }

    private static void drawLogo(
        Context context,
        Canvas canvas,
        ProviderUsage provider,
        float x,
        float y,
        float size,
        float scale
    ) {
        Drawable logo = context.getDrawable(logoResource(provider.id));
        if (logo == null) return;

        int iconSize = Math.round(15f * scale);
        int left = Math.round(x + 31f * scale - iconSize / 2f);
        int top = Math.round(y + 27f * scale - iconSize / 2f);
        logo.setBounds(left, top, left + iconSize, top + iconSize);
        logo.draw(canvas);
    }

    private static int logoResource(String providerId) {
        if ("codex".equals(providerId)) return R.drawable.logo_codex;
        if ("copilot".equals(providerId)) return R.drawable.logo_copilot;
        if ("openrouter".equals(providerId)) return R.drawable.logo_openrouter;
        if ("kimi".equals(providerId)) return R.drawable.logo_kimi;
        return R.drawable.logo_unknown;
    }

    private static int gaugeColor(int percentUsed) {
        if (percentUsed >= 90) return Color.rgb(255, 76, 72);
        if (percentUsed >= 75) return Color.rgb(255, 184, 76);
        return Color.rgb(48, 221, 111);
    }
}
