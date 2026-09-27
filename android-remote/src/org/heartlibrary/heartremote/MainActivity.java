package org.heartlibrary.heartremote;

import android.app.Activity;
import android.os.Build;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.view.View;
import android.view.Window;
import android.view.inputmethod.InputMethodManager;

import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.atomic.AtomicBoolean;

// Activity back navigation and bar colors keep the API 26-compatible behavior.
@SuppressWarnings("deprecation")
public final class MainActivity extends Activity {
    private static final long HEARTBEAT_INTERVAL_MS = 3500L;

    private final ExecutorService networkExecutor = Executors.newSingleThreadExecutor();
    private final Handler mainHandler = new Handler(Looper.getMainLooper());
    private final AtomicBoolean requestInFlight = new AtomicBoolean();

    private final Runnable heartbeatTask = new Runnable() {
        @Override
        public void run() {
            if (!active) return;
            ConnectionStore.SavedConnection connection = connectionStore.load();
            if (connection.isComplete()) checkHeartbeat(connection, false);
            mainHandler.postDelayed(this, HEARTBEAT_INTERVAL_MS);
        }
    };

    private ConnectionStore connectionStore;
    private HeartApiClient apiClient;
    private HeartDiscoveryClient discoveryClient;
    private RemoteWebViewController remoteController;
    private ConnectionScreen screen;
    private volatile boolean active;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        configureWindow();
        connectionStore = new ConnectionStore(this);
        apiClient = new HeartApiClient();
        discoveryClient = new HeartDiscoveryClient(this);
        remoteController = new RemoteWebViewController(this);
        showConnectionScreen();
        restoreConnection();
    }

    @Override
    protected void onStart() {
        super.onStart();
        active = true;
        remoteController.onHostStart();
        mainHandler.removeCallbacks(heartbeatTask);
        mainHandler.post(heartbeatTask);
    }

    @Override
    protected void onStop() {
        active = false;
        mainHandler.removeCallbacks(heartbeatTask);
        remoteController.onHostStop();
        super.onStop();
    }

    @Override
    protected void onDestroy() {
        remoteController.close();
        networkExecutor.shutdownNow();
        super.onDestroy();
    }

    private void configureWindow() {
        Window window = getWindow();
        window.setStatusBarColor(HeartTheme.BG);
        window.setNavigationBarColor(HeartTheme.BG);
    }

    private void showConnectionScreen() {
        screen = new ConnectionScreen(this);
        screen.discoverButton.setOnClickListener(view -> discoverHeart());
        screen.connectButton.setOnClickListener(view -> pairWithHeart());
        screen.openRemoteButton.setOnClickListener(view -> showRemote());
        screen.forgetButton.setOnClickListener(view -> forgetConnection());
        setContentView(screen.root);
    }

    private void restoreConnection() {
        ConnectionStore.SavedConnection connection = connectionStore.load();
        screen.showSavedConnection(connection);
        if (!connection.isComplete()) return;

        screen.setStatus(ConnectionScreen.State.CONNECTING, "저장된 HEART 확인 중", connection.baseUrl);
        checkHeartbeat(connection, true);
    }

    private void discoverHeart() {
        if (!beginRequest()) return;
        screen.setBusy(true, "같은 Wi-Fi에서 HEART를 찾는 중...");
        networkExecutor.execute(() -> {
            try {
                HeartDiscoveryClient.FoundHost host = discoveryClient.discover();
                onUi(() -> {
                    screen.addressInput.setText(host.authority());
                    screen.foundHost.setText("찾음 · " + host.name + " · " + host.address);
                    screen.foundHost.setTextColor(HeartTheme.GREEN);
                    screen.setStatus(
                        ConnectionScreen.State.READY,
                        "HEART를 찾았습니다",
                        "노트북의 6자리 코드를 입력하세요."
                    );
                    screen.codeInput.requestFocus();
                    screen.setBusy(false, null);
                });
            } catch (Exception error) {
                onUi(() -> {
                    screen.foundHost.setText("자동으로 찾지 못했습니다. 아래에 노트북 주소를 입력해 주세요.");
                    screen.foundHost.setTextColor(HeartTheme.AMBER);
                    screen.setStatus(
                        ConnectionScreen.State.OFFLINE,
                        "HEART를 찾지 못함",
                        ErrorMessage.readable(error)
                    );
                    screen.setBusy(false, null);
                });
            } finally {
                endRequest();
            }
        });
    }

    private void pairWithHeart() {
        if (requestInFlight.get()) return;
        String code = screen.codeInput.getText().toString().trim();
        if (!code.matches("\\d{6}")) {
            screen.setStatus(
                ConnectionScreen.State.ERROR,
                "코드를 확인해 주세요",
                "노트북 HEART의 6자리 숫자를 입력해야 합니다."
            );
            screen.codeInput.requestFocus();
            return;
        }

        final String baseUrl;
        try {
            baseUrl = apiClient.normalizeAddress(screen.addressInput.getText().toString());
        } catch (Exception error) {
            screen.setStatus(ConnectionScreen.State.ERROR, "주소를 확인해 주세요", ErrorMessage.readable(error));
            screen.addressInput.requestFocus();
            return;
        }
        if (!beginRequest()) return;

        hideKeyboard();
        screen.setBusy(true, "노트북 HEART에 연결하는 중...");
        screen.setStatus(ConnectionScreen.State.CONNECTING, "페어링 중", baseUrl);
        networkExecutor.execute(() -> {
            try {
                HeartApiClient.PairResult result = apiClient.pair(baseUrl, code, deviceName());
                connectionStore.save(baseUrl, result.token);
                onUi(() -> {
                    screen.codeInput.setText("");
                    screen.setStatus(
                        ConnectionScreen.State.CONNECTED,
                        "HEART 연결 완료",
                        result.hostName + " · " + baseUrl
                    );
                    screen.showConnectedActions(true);
                    screen.setBusy(false, null);
                    showRemote();
                });
            } catch (Exception error) {
                onUi(() -> {
                    screen.setStatus(ConnectionScreen.State.ERROR, "연결 실패", ErrorMessage.readable(error));
                    screen.setBusy(false, null);
                });
            } finally {
                endRequest();
            }
        });
    }

    private void checkHeartbeat(ConnectionStore.SavedConnection connection, boolean openOnSuccess) {
        if (!beginRequest()) return;
        networkExecutor.execute(() -> {
            try {
                String hostName = apiClient.heartbeat(connection.baseUrl, connection.token);
                onUi(() -> {
                    screen.showConnected(hostName, hostName + " · 실시간 응답 정상");
                    if (openOnSuccess) showRemote();
                });
            } catch (Exception error) {
                if (openOnSuccess || active) {
                    onUi(() -> screen.setStatus(
                        ConnectionScreen.State.OFFLINE,
                        "HEART 응답 대기 중",
                        ErrorMessage.readable(error)
                    ));
                }
            } finally {
                endRequest();
            }
        });
    }

    private void showRemote() {
        ConnectionStore.SavedConnection connection = connectionStore.load();
        if (!connection.isComplete()) {
            screen.setStatus(
                ConnectionScreen.State.ERROR,
                "연결 정보가 없습니다",
                "HEART와 다시 페어링해 주세요."
            );
            return;
        }
        remoteController.open(connection.baseUrl, connection.token);
    }

    private void forgetConnection() {
        connectionStore.clear();
        screen.clear();
    }

    private boolean beginRequest() {
        return requestInFlight.compareAndSet(false, true);
    }

    private void endRequest() {
        requestInFlight.set(false);
    }

    private void onUi(Runnable action) {
        runOnUiThread(() -> {
            if (!isFinishing()) action.run();
        });
    }

    private void hideKeyboard() {
        View focused = getCurrentFocus();
        if (focused == null) return;
        InputMethodManager input = (InputMethodManager) getSystemService(INPUT_METHOD_SERVICE);
        if (input != null) input.hideSoftInputFromWindow(focused.getWindowToken(), 0);
    }

    private String deviceName() {
        String manufacturer = Build.MANUFACTURER == null ? "Samsung" : Build.MANUFACTURER;
        String model = Build.MODEL == null ? "Android" : Build.MODEL;
        return (manufacturer + " " + model).trim();
    }

    @Override
    public void onWindowFocusChanged(boolean hasFocus) {
        super.onWindowFocusChanged(hasFocus);
        remoteController.restoreImmersiveMode(hasFocus);
    }

    @Override
    public void onBackPressed() {
        if (!remoteController.isOpen()) {
            super.onBackPressed();
            return;
        }
        if (remoteController.handleBack()) return;
        showConnectionScreen();
        restoreConnection();
    }
}
