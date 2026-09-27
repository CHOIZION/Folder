package org.heartlibrary.heartremote;

import android.app.Activity;
import android.content.pm.ActivityInfo;
import android.view.View;
import android.view.WindowManager;
import android.view.inputmethod.InputMethodManager;
import android.webkit.JavascriptInterface;
import android.webkit.WebChromeClient;
import android.webkit.WebSettings;
import android.webkit.WebView;
import android.webkit.WebViewClient;

import org.json.JSONObject;

// Legacy system UI flags are required to preserve immersive mode on API 26-29.
@SuppressWarnings("deprecation")
final class RemoteWebViewController {
    private final Activity activity;
    private WebView webView;
    private H264FullscreenController h264Controller;
    private String baseUrl;
    private String token;
    private boolean fullscreen;

    RemoteWebViewController(Activity activity) {
        this.activity = activity;
    }

    boolean isOpen() {
        return webView != null;
    }

    void open(String baseUrl, String token) {
        close();
        this.baseUrl = baseUrl;
        this.token = token;
        webView = new WebView(activity);
        webView.setBackgroundColor(HeartTheme.BG);
        // The remote page now receives a 60 Hz MJPEG image stream. Keep the
        // compositor on the GPU and avoid retaining obsolete image frames.
        webView.setLayerType(View.LAYER_TYPE_HARDWARE, null);

        WebSettings settings = webView.getSettings();
        settings.setJavaScriptEnabled(true);
        settings.setDomStorageEnabled(true);
        settings.setCacheMode(WebSettings.LOAD_NO_CACHE);
        settings.setLoadsImagesAutomatically(true);
        settings.setBlockNetworkImage(false);
        settings.setOffscreenPreRaster(true);
        settings.setBuiltInZoomControls(false);
        settings.setDisplayZoomControls(false);
        settings.setMediaPlaybackRequiresUserGesture(false);
        webView.setWebViewClient(new WebViewClient());
        webView.setWebChromeClient(new WebChromeClient());
        webView.addJavascriptInterface(new RemoteBridge(), "HeartAndroid");
        activity.setContentView(webView);
        webView.loadUrl(baseUrl + "/remote#token=" + token);
    }

    boolean handleBack() {
        if (webView == null) return false;
        if (fullscreen) {
            exitFullscreen();
            return true;
        }
        if (webView.canGoBack()) {
            webView.goBack();
            return true;
        }
        close();
        return false;
    }

    void restoreImmersiveMode(boolean hasFocus) {
        if (hasFocus && fullscreen) applyImmersiveMode();
    }

    void onHostStart() {
        if (h264Controller != null) h264Controller.resume();
        else if (webView != null) webView.onResume();
    }

    void onHostStop() {
        if (h264Controller != null) h264Controller.pause();
        if (webView != null) webView.onPause();
    }

    void close() {
        if (fullscreen) exitFullscreen();
        closeH264Controller();
        if (webView == null) return;
        webView.stopLoading();
        webView.loadUrl("about:blank");
        webView.removeJavascriptInterface("HeartAndroid");
        webView.destroy();
        webView = null;
        baseUrl = null;
        token = null;
    }

    private void enterFullscreen(boolean swipeInverted) {
        if (webView == null || baseUrl == null || token == null) return;
        fullscreen = true;
        activity.setRequestedOrientation(ActivityInfo.SCREEN_ORIENTATION_SENSOR_LANDSCAPE);
        activity.getWindow().addFlags(WindowManager.LayoutParams.FLAG_FULLSCREEN);
        applyImmersiveMode();
        webView.onPause();
        closeH264Controller();
        h264Controller = new H264FullscreenController(
            activity,
            baseUrl,
            token,
            swipeInverted,
            new H264FullscreenController.Listener() {
                @Override
                public void onExit() {
                    exitFullscreen();
                }

                @Override
                public void onFallback(String message) {
                    fallbackToMjpeg(message);
                }
            }
        );
        h264Controller.open();
    }

    private void exitFullscreen() {
        closeH264Controller();
        fullscreen = false;
        if (webView != null) {
            activity.setContentView(webView);
            webView.onResume();
            webView.evaluateJavascript(
                "window.heartExitFullscreenFromAndroid && window.heartExitFullscreenFromAndroid()",
                null
            );
        }
        activity.setRequestedOrientation(ActivityInfo.SCREEN_ORIENTATION_UNSPECIFIED);
        activity.getWindow().clearFlags(WindowManager.LayoutParams.FLAG_FULLSCREEN);
        activity.getWindow().getDecorView().setSystemUiVisibility(View.SYSTEM_UI_FLAG_VISIBLE);
        activity.getWindow().setStatusBarColor(HeartTheme.BG);
        activity.getWindow().setNavigationBarColor(HeartTheme.BG);
    }

    private void fallbackToMjpeg(String message) {
        closeH264Controller();
        if (webView == null) return;
        activity.setContentView(webView);
        webView.onResume();
        webView.evaluateJavascript(
            "window.heartH264Fallback && window.heartH264Fallback(" + JSONObject.quote(message) + ")",
            null
        );
        applyImmersiveMode();
    }

    private void closeH264Controller() {
        H264FullscreenController active = h264Controller;
        h264Controller = null;
        if (active != null) active.close();
    }

    private void applyImmersiveMode() {
        activity.getWindow().getDecorView().setSystemUiVisibility(
            View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY
                | View.SYSTEM_UI_FLAG_FULLSCREEN
                | View.SYSTEM_UI_FLAG_HIDE_NAVIGATION
                | View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN
                | View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION
                | View.SYSTEM_UI_FLAG_LAYOUT_STABLE
        );
    }

    private void showKeyboard() {
        if (webView == null) return;
        webView.requestFocus();
        webView.postDelayed(() -> {
            if (webView == null) return;
            InputMethodManager input = (InputMethodManager) activity.getSystemService(
                Activity.INPUT_METHOD_SERVICE
            );
            if (input != null) input.showSoftInput(webView, InputMethodManager.SHOW_IMPLICIT);
        }, 80L);
    }

    private final class RemoteBridge {
        @JavascriptInterface
        public void enterFullscreen(boolean swipeInverted) {
            activity.runOnUiThread(
                () -> RemoteWebViewController.this.enterFullscreen(swipeInverted)
            );
        }

        @JavascriptInterface
        public void exitFullscreen() {
            activity.runOnUiThread(() -> RemoteWebViewController.this.exitFullscreen());
        }

        @JavascriptInterface
        public void showKeyboard() {
            activity.runOnUiThread(() -> RemoteWebViewController.this.showKeyboard());
        }
    }
}
