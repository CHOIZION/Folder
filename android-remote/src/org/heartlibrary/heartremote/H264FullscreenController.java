package org.heartlibrary.heartremote;

import android.app.Activity;
import android.app.AlertDialog;
import android.graphics.Color;
import android.graphics.drawable.GradientDrawable;
import android.os.Handler;
import android.os.Looper;
import android.text.InputType;
import android.view.Gravity;
import android.view.SurfaceHolder;
import android.view.SurfaceView;
import android.view.View;
import android.view.WindowManager;
import android.view.inputmethod.InputMethodManager;
import android.widget.Button;
import android.widget.EditText;
import android.widget.FrameLayout;
import android.widget.LinearLayout;
import android.widget.TextView;

final class H264FullscreenController {
    private static final long CONTROLS_AUTO_HIDE_MS = 5_000L;

    interface Listener {
        void onExit();
        void onFallback(String message);
    }

    private final Activity activity;
    private final String baseUrl;
    private final String token;
    private final Listener listener;
    private final Handler mainHandler = new Handler(Looper.getMainLooper());
    private final Runnable hideControlsTask = this::hideControls;
    private final float density;
    private final RemoteInputDispatcher input;

    private FrameLayout root;
    private SurfaceView surfaceView;
    private RemoteGestureController gestures;
    private View topBar;
    private Button menuButton;
    private TextView status;
    private Button invertButton;
    private H264StreamClient streamClient;
    private int videoWidth = 16;
    private int videoHeight = 9;
    private boolean swipeInverted;
    private boolean closed;
    private boolean fallbackDelivered;

    H264FullscreenController(
        Activity activity,
        String baseUrl,
        String token,
        boolean swipeInverted,
        Listener listener
    ) {
        this.activity = activity;
        this.baseUrl = baseUrl;
        this.token = token;
        this.swipeInverted = swipeInverted;
        this.listener = listener;
        this.density = activity.getResources().getDisplayMetrics().density;
        this.input = new RemoteInputDispatcher(baseUrl, token);
    }

    void open() {
        root = new FrameLayout(activity);
        root.setBackgroundColor(Color.BLACK);
        surfaceView = new SurfaceView(activity);
        surfaceView.setKeepScreenOn(true);
        gestures = new RemoteGestureController(
            surfaceView,
            input,
            mainHandler,
            density,
            () -> swipeInverted
        );
        surfaceView.setOnTouchListener(gestures);
        root.addView(surfaceView, matchFrame());
        root.addOnLayoutChangeListener(
            (view, left, top, right, bottom, oldLeft, oldTop, oldRight, oldBottom) ->
                resizeSurfaceToVideo()
        );
        topBar = buildTopBar();
        root.addView(topBar, topBarParams());
        menuButton = button("메뉴");
        menuButton.setOnClickListener(view -> showControls());
        root.addView(menuButton, menuButtonParams());
        root.addView(buildHint(), bottomHintParams());
        activity.setContentView(root);
        showControls();

        surfaceView.getHolder().addCallback(new SurfaceHolder.Callback() {
            @Override
            public void surfaceCreated(SurfaceHolder holder) {
                startStream(holder);
            }

            @Override
            public void surfaceChanged(SurfaceHolder holder, int format, int width, int height) { }

            @Override
            public void surfaceDestroyed(SurfaceHolder holder) {
                stopStream();
            }
        });
    }

    void close() {
        if (closed) return;
        closed = true;
        mainHandler.removeCallbacksAndMessages(null);
        stopStream();
        if (gestures != null) gestures.cancel();
        input.close();
        root = null;
        surfaceView = null;
        topBar = null;
        menuButton = null;
        gestures = null;
    }

    void pause() {
        if (closed) return;
        stopStream();
        if (gestures != null) gestures.cancel();
    }

    void resume() {
        if (closed || surfaceView == null) return;
        SurfaceHolder holder = surfaceView.getHolder();
        if (holder.getSurface().isValid()) startStream(holder);
    }

    private void startStream(SurfaceHolder holder) {
        stopStream();
        if (closed || !holder.getSurface().isValid()) return;
        streamClient = new H264StreamClient(baseUrl, token, holder.getSurface(), new H264StreamClient.Listener() {
            @Override
            public void onStreamInfo(H264StreamClient.StreamInfo info) {
                activity.runOnUiThread(() -> {
                    if (closed) return;
                    videoWidth = info.width;
                    videoHeight = info.height;
                    surfaceView.getHolder().setFixedSize(videoWidth, videoHeight);
                    resizeSurfaceToVideo();
                    String acceleration = info.hardware ? "HW" : "SW 폴백";
                    status.setText(
                        info.width + "×" + info.height + " · " + info.fps + "fps · "
                            + (info.bitrate / 1_000_000) + "Mbps · " + acceleration
                    );
                });
            }

            @Override
            public void onStats(H264StreamClient.StreamStats stats) {
                activity.runOnUiThread(() -> {
                    if (closed) return;
                    status.setText(
                        "H.264 수신 " + stats.receivedFps + " · 표시 " + stats.displayedFps
                            + " fps · 약 " + stats.estimatedLatencyMs + "ms"
                    );
                });
            }

            @Override
            public void onRecovering(String message, long retryDelayMs) {
                activity.runOnUiThread(() -> {
                    if (!closed) status.setText("재연결 중 · " + message);
                });
            }

            @Override
            public void onFatalError(String message) {
                activity.runOnUiThread(() -> {
                    if (closed || fallbackDelivered) return;
                    fallbackDelivered = true;
                    listener.onFallback(message);
                });
            }
        });
        streamClient.start();
    }

    private void stopStream() {
        H264StreamClient active = streamClient;
        streamClient = null;
        if (active != null) active.stop();
    }

    private View buildTopBar() {
        LinearLayout bar = new LinearLayout(activity);
        bar.setOrientation(LinearLayout.HORIZONTAL);
        bar.setGravity(Gravity.CENTER_VERTICAL);
        bar.setPadding(dp(10), dp(8), dp(10), dp(8));
        bar.setBackgroundColor(Color.argb(184, 9, 9, 11));

        Button exit = button("← 돌아가기");
        exit.setOnClickListener(view -> listener.onExit());
        bar.addView(exit, new LinearLayout.LayoutParams(dp(104), dp(42)));

        status = new TextView(activity);
        status.setText("H.264 하드웨어 스트림 연결 중…");
        status.setTextColor(Color.WHITE);
        status.setTextSize(11);
        status.setGravity(Gravity.CENTER);
        status.setSingleLine(true);
        bar.addView(status, new LinearLayout.LayoutParams(0, dp(42), 1f));

        invertButton = button("");
        updateInvertLabel();
        invertButton.setOnClickListener(view -> {
            swipeInverted = !swipeInverted;
            updateInvertLabel();
        });
        bar.addView(invertButton, new LinearLayout.LayoutParams(dp(116), dp(42)));

        Button keyboard = button("⌨ 문자");
        keyboard.setOnClickListener(view -> showKeyboardDialog());
        LinearLayout.LayoutParams keyboardParams = new LinearLayout.LayoutParams(dp(86), dp(42));
        keyboardParams.leftMargin = dp(7);
        bar.addView(keyboard, keyboardParams);

        Button hide = button("화면 터치");
        hide.setOnClickListener(view -> hideControls());
        LinearLayout.LayoutParams hideParams = new LinearLayout.LayoutParams(dp(92), dp(42));
        hideParams.leftMargin = dp(7);
        bar.addView(hide, hideParams);
        return bar;
    }

    private TextView buildHint() {
        TextView hint = new TextView(activity);
        hint.setText("빠른 좌우: 방향키 · 상하: 스크롤 · 길게 누른 뒤 이동: 드래그 · 두 손가락: 스크롤");
        hint.setTextColor(Color.WHITE);
        hint.setTextSize(10);
        hint.setGravity(Gravity.CENTER);
        hint.setPadding(dp(12), dp(7), dp(12), dp(7));
        hint.setBackgroundColor(Color.argb(155, 9, 9, 11));
        mainHandler.postDelayed(() -> {
            if (!closed) hint.animate().alpha(0f).setDuration(500).start();
        }, 5000L);
        return hint;
    }

    private void showKeyboardDialog() {
        EditText editor = new EditText(activity);
        editor.setSingleLine(false);
        editor.setMaxLines(4);
        editor.setInputType(InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_FLAG_MULTI_LINE);
        editor.setHint("노트북에 보낼 한글·영문 입력");
        int padding = dp(18);
        FrameLayout wrapper = new FrameLayout(activity);
        wrapper.setPadding(padding, 0, padding, 0);
        wrapper.addView(
            editor,
            new FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.WRAP_CONTENT
            )
        );

        AlertDialog dialog = new AlertDialog.Builder(activity)
            .setTitle("노트북 문자 입력")
            .setView(wrapper)
            .setNegativeButton("취소", null)
            .setPositiveButton("전송", (target, which) -> {
                String text = editor.getText().toString();
                if (!text.isEmpty()) input.text(text);
            })
            .create();
        dialog.setOnShowListener(ignored -> {
            editor.requestFocus();
            if (dialog.getWindow() != null) {
                dialog.getWindow().setSoftInputMode(WindowManager.LayoutParams.SOFT_INPUT_STATE_ALWAYS_VISIBLE);
            }
            InputMethodManager keyboard = (InputMethodManager) activity.getSystemService(Activity.INPUT_METHOD_SERVICE);
            if (keyboard != null) editor.postDelayed(
                () -> keyboard.showSoftInput(editor, InputMethodManager.SHOW_IMPLICIT),
                100L
            );
        });
        dialog.show();
    }

    private void updateInvertLabel() {
        invertButton.setText("반전 " + (swipeInverted ? "켬" : "끔"));
    }

    private void showControls() {
        if (closed || topBar == null || menuButton == null) return;
        mainHandler.removeCallbacks(hideControlsTask);
        topBar.setVisibility(View.VISIBLE);
        menuButton.setVisibility(View.GONE);
        mainHandler.postDelayed(hideControlsTask, CONTROLS_AUTO_HIDE_MS);
    }

    private void hideControls() {
        if (closed || topBar == null || menuButton == null) return;
        mainHandler.removeCallbacks(hideControlsTask);
        topBar.setVisibility(View.GONE);
        menuButton.setVisibility(View.VISIBLE);
    }

    private void resizeSurfaceToVideo() {
        if (root == null || surfaceView == null || videoWidth <= 0 || videoHeight <= 0) return;
        int availableWidth = root.getWidth();
        int availableHeight = root.getHeight();
        if (availableWidth <= 0 || availableHeight <= 0) return;

        int width = availableWidth;
        int height = Math.round((float) availableWidth * videoHeight / videoWidth);
        if (height > availableHeight) {
            height = availableHeight;
            width = Math.round((float) availableHeight * videoWidth / videoHeight);
        }

        FrameLayout.LayoutParams current = (FrameLayout.LayoutParams) surfaceView.getLayoutParams();
        if (current.width == width && current.height == height && current.gravity == Gravity.CENTER) {
            return;
        }
        FrameLayout.LayoutParams fitted = new FrameLayout.LayoutParams(width, height);
        fitted.gravity = Gravity.CENTER;
        surfaceView.setLayoutParams(fitted);
    }

    private Button button(String label) {
        Button button = new Button(activity);
        button.setAllCaps(false);
        button.setText(label);
        button.setTextColor(Color.WHITE);
        button.setTextSize(11);
        button.setPadding(dp(7), 0, dp(7), 0);
        GradientDrawable background = new GradientDrawable();
        background.setColor(Color.rgb(39, 39, 42));
        background.setStroke(dp(1), Color.rgb(82, 82, 91));
        background.setCornerRadius(dp(20));
        button.setBackground(background);
        return button;
    }

    private FrameLayout.LayoutParams matchFrame() {
        return new FrameLayout.LayoutParams(
            FrameLayout.LayoutParams.MATCH_PARENT,
            FrameLayout.LayoutParams.MATCH_PARENT
        );
    }

    private FrameLayout.LayoutParams topBarParams() {
        FrameLayout.LayoutParams params = new FrameLayout.LayoutParams(
            FrameLayout.LayoutParams.MATCH_PARENT,
            dp(58)
        );
        params.gravity = Gravity.TOP;
        return params;
    }

    private FrameLayout.LayoutParams menuButtonParams() {
        FrameLayout.LayoutParams params = new FrameLayout.LayoutParams(dp(72), dp(42));
        params.gravity = Gravity.TOP | Gravity.START;
        params.leftMargin = dp(8);
        params.topMargin = dp(8);
        return params;
    }

    private FrameLayout.LayoutParams bottomHintParams() {
        FrameLayout.LayoutParams params = new FrameLayout.LayoutParams(
            FrameLayout.LayoutParams.WRAP_CONTENT,
            dp(36)
        );
        params.gravity = Gravity.BOTTOM | Gravity.CENTER_HORIZONTAL;
        params.bottomMargin = dp(10);
        return params;
    }

    private int dp(int value) {
        return Math.round(value * density);
    }

}
