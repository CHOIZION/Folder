package org.heartlibrary.heartremote;

import android.app.Activity;
import android.graphics.Color;
import android.graphics.Typeface;
import android.graphics.drawable.GradientDrawable;
import android.text.InputType;
import android.view.Gravity;
import android.view.View;
import android.widget.Button;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.Space;
import android.widget.TextView;

final class ConnectionScreen {
    enum State { OFFLINE, CONNECTING, READY, CONNECTED, ERROR }

    private final Activity activity;
    private final float density;

    final ScrollView root;
    final EditText addressInput;
    final EditText codeInput;
    final Button discoverButton;
    final Button connectButton;
    final Button forgetButton;
    final Button openRemoteButton;
    final TextView foundHost;

    private final TextView statusDot;
    private final TextView statusTitle;
    private final TextView statusDetail;
    private final TextView footerStatus;

    ConnectionScreen(Activity activity) {
        this.activity = activity;
        density = activity.getResources().getDisplayMetrics().density;

        root = new ScrollView(activity);
        root.setFillViewport(true);
        root.setBackgroundColor(HeartTheme.BG);

        LinearLayout content = vertical();
        content.setPadding(dp(22), dp(28), dp(22), dp(30));
        root.addView(content, matchWrap());

        TextView kicker = text("ANDROID COMPANION", 11, HeartTheme.RED, true);
        kicker.setLetterSpacing(0.18f);
        content.addView(kicker, matchHeight(dp(22)));
        addHeader(content);

        LinearLayout statusCard = vertical();
        statusCard.setPadding(dp(18), dp(17), dp(18), dp(17));
        statusCard.setBackground(rounded(HeartTheme.PANEL, HeartTheme.BORDER, 14));
        LinearLayout statusRow = horizontal();
        statusRow.setGravity(Gravity.CENTER_VERTICAL);
        statusDot = text("●", 18, HeartTheme.INACTIVE, true);
        statusRow.addView(statusDot, new LinearLayout.LayoutParams(dp(34), dp(34)));
        LinearLayout statusText = vertical();
        statusTitle = text("연결되지 않음", 16, HeartTheme.TEXT, true);
        statusDetail = text("노트북에서 HEART를 먼저 실행하세요.", 12, HeartTheme.MUTED, false);
        statusText.addView(statusTitle, matchHeight(dp(25)));
        statusText.addView(statusDetail, matchHeight(dp(22)));
        statusRow.addView(statusText, new LinearLayout.LayoutParams(0, dp(50), 1f));
        statusCard.addView(statusRow, matchHeight(dp(54)));
        content.addView(statusCard, matchHeight(dp(88)));

        content.addView(space(18));
        content.addView(sectionLabel("01 · 노트북 HEART 찾기"), matchHeight(dp(30)));
        discoverButton = actionButton("HEART 자동 찾기", false);
        content.addView(discoverButton, matchHeight(dp(52)));
        foundHost = text("같은 Wi-Fi 자동 찾기 또는 Tailscale 주소를 사용하세요.", 12, HeartTheme.MUTED, false);
        foundHost.setPadding(dp(3), dp(9), dp(3), dp(3));
        content.addView(foundHost, matchHeight(dp(42)));
        content.addView(fieldLabel("노트북 주소 · LAN IP 또는 Tailscale 주소"), matchHeight(dp(28)));
        addressInput = field(
            "예: 192.168.0.10 또는 my-pc.tailnet.ts.net",
            InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_VARIATION_URI
        );
        content.addView(addressInput, matchHeight(dp(52)));

        content.addView(space(20));
        content.addView(sectionLabel("02 · 6자리 코드로 연결"), matchHeight(dp(30)));
        codeInput = field("노트북 HEART에 표시된 코드", InputType.TYPE_CLASS_NUMBER);
        codeInput.setLetterSpacing(0.18f);
        codeInput.setTextSize(20);
        content.addView(codeInput, matchHeight(dp(56)));
        content.addView(space(10));

        connectButton = actionButton("HEART와 연결하기", true);
        content.addView(connectButton, matchHeight(dp(54)));
        openRemoteButton = actionButton("HEART 리모컨 열기", true);
        openRemoteButton.setVisibility(View.GONE);
        LinearLayout.LayoutParams remoteParams = matchHeight(dp(52));
        remoteParams.topMargin = dp(10);
        content.addView(openRemoteButton, remoteParams);
        forgetButton = actionButton("저장된 연결 지우기", false);
        forgetButton.setVisibility(View.GONE);
        LinearLayout.LayoutParams forgetParams = matchHeight(dp(48));
        forgetParams.topMargin = dp(10);
        content.addView(forgetButton, forgetParams);

        content.addView(space(22));
        addInfoCard(content);
        footerStatus = text("HEART Remote · LAN / Tailscale", 11, HeartTheme.INACTIVE, false);
        footerStatus.setGravity(Gravity.CENTER);
        LinearLayout.LayoutParams footerParams = matchHeight(dp(45));
        footerParams.topMargin = dp(14);
        content.addView(footerStatus, footerParams);
    }

    void showSavedConnection(ConnectionStore.SavedConnection connection) {
        if (!connection.baseUrl.isEmpty()) {
            addressInput.setText(connection.baseUrl.replace("http://", ""));
        }
        showConnectedActions(connection.isComplete());
    }

    void showConnectedActions(boolean visible) {
        int visibility = visible ? View.VISIBLE : View.GONE;
        forgetButton.setVisibility(visibility);
        openRemoteButton.setVisibility(visibility);
    }

    void setBusy(boolean busy, String message) {
        discoverButton.setEnabled(!busy);
        connectButton.setEnabled(!busy);
        discoverButton.setAlpha(busy ? 0.55f : 1f);
        connectButton.setAlpha(busy ? 0.55f : 1f);
        if (message != null) foundHost.setText(message);
    }

    void setStatus(State state, String title, String detail) {
        int color;
        switch (state) {
            case CONNECTED: color = HeartTheme.GREEN; break;
            case CONNECTING:
            case READY: color = HeartTheme.AMBER; break;
            case ERROR: color = HeartTheme.RED; break;
            default: color = HeartTheme.INACTIVE; break;
        }
        statusDot.setTextColor(color);
        statusTitle.setText(title);
        statusDetail.setText(detail);
    }

    void showConnected(String hostName, String detail) {
        setStatus(State.CONNECTED, "HEART 연결됨", detail);
        footerStatus.setText("연결됨 · " + hostName);
        showConnectedActions(true);
    }

    void clear() {
        addressInput.setText("");
        codeInput.setText("");
        showConnectedActions(false);
        footerStatus.setText("HEART Remote · LAN / Tailscale");
        setStatus(State.OFFLINE, "연결되지 않음", "노트북에서 HEART를 실행한 뒤 다시 연결하세요.");
    }

    private void addHeader(LinearLayout content) {
        LinearLayout titleRow = horizontal();
        titleRow.setGravity(Gravity.CENTER_VERTICAL);
        TextView heart = text("♥", 38, HeartTheme.RED, true);
        titleRow.addView(heart, new LinearLayout.LayoutParams(dp(48), dp(54)));
        TextView title = text("HEART\nREMOTE", 25, HeartTheme.TEXT, true);
        title.setLineSpacing(0f, 0.88f);
        titleRow.addView(title, new LinearLayout.LayoutParams(0, dp(64), 1f));
        content.addView(titleRow, matchHeight(dp(74)));
    }

    private void addInfoCard(LinearLayout content) {
        LinearLayout info = vertical();
        info.setPadding(dp(16), dp(14), dp(16), dp(14));
        info.setBackground(rounded(Color.rgb(18, 18, 22), HeartTheme.FIELD, 12));
        TextView title = text("HEART Remote Control", 13, HeartTheme.TEXT, true);
        TextView body = text(
            "연결 뒤에는 작품 목록·검색·실행과 노트북 실시간 화면, 스와이프, 터치 마우스, 스크롤, 키보드 입력을 한 앱에서 사용할 수 있습니다. 자동 찾기는 같은 Wi-Fi에서만 동작합니다.",
            12, HeartTheme.MUTED, false
        );
        body.setLineSpacing(dp(4), 1f);
        info.addView(title, matchHeight(dp(27)));
        info.addView(body, matchWrap());
        content.addView(info, matchWrap());
    }

    private LinearLayout vertical() {
        LinearLayout layout = new LinearLayout(activity);
        layout.setOrientation(LinearLayout.VERTICAL);
        return layout;
    }

    private LinearLayout horizontal() {
        LinearLayout layout = new LinearLayout(activity);
        layout.setOrientation(LinearLayout.HORIZONTAL);
        return layout;
    }

    private TextView text(String value, int sizeSp, int color, boolean bold) {
        TextView view = new TextView(activity);
        view.setText(value);
        view.setTextSize(sizeSp);
        view.setTextColor(color);
        view.setGravity(Gravity.CENTER_VERTICAL);
        if (bold) view.setTypeface(Typeface.DEFAULT, Typeface.BOLD);
        return view;
    }

    private TextView sectionLabel(String value) {
        TextView view = text(value, 12, HeartTheme.MUTED, true);
        view.setLetterSpacing(0.06f);
        return view;
    }

    private TextView fieldLabel(String value) {
        TextView view = text(value, 11, HeartTheme.DIMMED, true);
        view.setPadding(dp(3), 0, 0, 0);
        return view;
    }

    private EditText field(String hint, int inputType) {
        EditText field = new EditText(activity);
        field.setHint(hint);
        field.setHintTextColor(HeartTheme.DIMMED);
        field.setTextColor(HeartTheme.TEXT);
        field.setTextSize(15);
        field.setSingleLine(true);
        field.setInputType(inputType);
        field.setPadding(dp(16), 0, dp(16), 0);
        field.setBackground(rounded(HeartTheme.FIELD, HeartTheme.BORDER, 10));
        return field;
    }

    private Button actionButton(String label, boolean primary) {
        Button button = new Button(activity);
        button.setAllCaps(false);
        button.setText(label);
        button.setTextSize(14);
        button.setTypeface(Typeface.DEFAULT, Typeface.BOLD);
        button.setTextColor(Color.WHITE);
        button.setGravity(Gravity.CENTER);
        button.setPadding(dp(12), 0, dp(12), 0);
        int color = primary ? HeartTheme.PRIMARY_RED : HeartTheme.FIELD;
        button.setBackground(rounded(color, primary ? color : HeartTheme.BORDER, 10));
        return button;
    }

    private GradientDrawable rounded(int fill, int stroke, int radiusDp) {
        GradientDrawable drawable = new GradientDrawable();
        drawable.setColor(fill);
        drawable.setCornerRadius(dp(radiusDp));
        drawable.setStroke(dp(1), stroke);
        return drawable;
    }

    private Space space(int heightDp) {
        Space space = new Space(activity);
        space.setLayoutParams(matchHeight(dp(heightDp)));
        return space;
    }

    private LinearLayout.LayoutParams matchHeight(int height) {
        return new LinearLayout.LayoutParams(LinearLayout.LayoutParams.MATCH_PARENT, height);
    }

    private LinearLayout.LayoutParams matchWrap() {
        return new LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT,
            LinearLayout.LayoutParams.WRAP_CONTENT
        );
    }

    private int dp(int value) {
        return Math.round(value * density);
    }
}
