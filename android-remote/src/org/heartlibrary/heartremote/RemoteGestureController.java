package org.heartlibrary.heartremote;

import android.os.Handler;
import android.view.MotionEvent;
import android.view.SurfaceView;
import android.view.View;

/** Translates touch gestures into bounded, coalesced remote input commands. */
final class RemoteGestureController implements View.OnTouchListener {
    interface SwipeDirection {
        boolean isInverted();
    }

    private static final long DRAG_DELAY_MS = 280L;

    private enum State {
        IDLE,
        PENDING,
        MOUSE,
        SWIPE,
        SCROLL,
        SCROLL_ENDING
    }

    private final SurfaceView surface;
    private final RemoteInputDispatcher input;
    private final Handler handler;
    private final float density;
    private final SwipeDirection swipeDirection;
    private final Runnable longPress = this::beginDrag;

    private State state = State.IDLE;
    private int primaryPointer = -1;
    private float startX;
    private float startY;
    private float lastX;
    private float lastY;
    private float lastSwipeX;
    private float lastSwipeY;
    private float scrollX;
    private float scrollY;

    RemoteGestureController(
        SurfaceView surface,
        RemoteInputDispatcher input,
        Handler handler,
        float density,
        SwipeDirection swipeDirection
    ) {
        this.surface = surface;
        this.input = input;
        this.handler = handler;
        this.density = density;
        this.swipeDirection = swipeDirection;
    }

    @Override
    public boolean onTouch(View view, MotionEvent event) {
        switch (event.getActionMasked()) {
            case MotionEvent.ACTION_DOWN:
                beginPointer(event);
                return true;
            case MotionEvent.ACTION_POINTER_DOWN:
                beginTwoFingerScroll(event);
                return true;
            case MotionEvent.ACTION_MOVE:
                move(event);
                return true;
            case MotionEvent.ACTION_POINTER_UP:
                if (state == State.SCROLL) state = State.SCROLL_ENDING;
                return true;
            case MotionEvent.ACTION_UP:
                finish(event, false);
                return true;
            case MotionEvent.ACTION_CANCEL:
                finish(event, true);
                return true;
            default:
                return true;
        }
    }

    void cancel() {
        handler.removeCallbacks(longPress);
        if (state == State.MOUSE) {
            sendButton("up", lastX, lastY);
        }
        reset();
    }

    private void beginPointer(MotionEvent event) {
        primaryPointer = event.getPointerId(0);
        startX = lastX = lastSwipeX = event.getX(0);
        startY = lastY = lastSwipeY = event.getY(0);
        state = State.PENDING;
        handler.removeCallbacks(longPress);
        handler.postDelayed(longPress, DRAG_DELAY_MS);
    }

    private void beginTwoFingerScroll(MotionEvent event) {
        handler.removeCallbacks(longPress);
        releaseDragIfActive();
        state = State.SCROLL;
        scrollX = centerX(event);
        scrollY = centerY(event);
        primaryPointer = -1;
    }

    private void beginDrag() {
        if (state != State.PENDING || primaryPointer < 0) return;
        state = State.MOUSE;
        sendMove(lastX, lastY);
        sendButton("down", lastX, lastY);
    }

    private void move(MotionEvent event) {
        if (state == State.SCROLL && event.getPointerCount() >= 2) {
            scroll(event);
            return;
        }
        if (primaryPointer < 0) return;
        int index = event.findPointerIndex(primaryPointer);
        if (index < 0) return;

        lastX = event.getX(index);
        lastY = event.getY(index);
        if (state == State.MOUSE) {
            sendMove(lastX, lastY);
        } else if (state == State.PENDING && emitSwipe(startX, startY, lastX, lastY)) {
            handler.removeCallbacks(longPress);
            state = State.SWIPE;
            lastSwipeX = lastX;
            lastSwipeY = lastY;
        } else if (state == State.SWIPE && emitSwipe(lastSwipeX, lastSwipeY, lastX, lastY)) {
            lastSwipeX = lastX;
            lastSwipeY = lastY;
        }
    }

    private void scroll(MotionEvent event) {
        float centerX = centerX(event);
        float centerY = centerY(event);
        float dx = centerX - scrollX;
        float dy = centerY - scrollY;
        scrollX = centerX;
        scrollY = centerY;
        if (Math.abs(dx) >= Math.abs(dy) && Math.abs(dx) > density) {
            input.wheel(Math.round(dx * 12f / density), true);
        } else if (Math.abs(dy) > density) {
            input.wheel(Math.round(-dy * 12f / density), false);
        }
    }

    private void finish(MotionEvent event, boolean cancelled) {
        handler.removeCallbacks(longPress);
        if (state == State.PENDING && !cancelled) {
            sendButton("click", event.getX(), event.getY());
        } else if (state == State.MOUSE) {
            sendButton("up", event.getX(), event.getY());
        }
        reset();
    }

    private void releaseDragIfActive() {
        if (state != State.MOUSE) return;
        sendButton("up", lastX, lastY);
    }

    private boolean emitSwipe(float fromX, float fromY, float toX, float toY) {
        float dx = toX - fromX;
        float dy = toY - fromY;
        if (Math.max(Math.abs(dx), Math.abs(dy)) < 42f * density) return false;

        boolean inverted = swipeDirection.isInverted();
        if (Math.abs(dx) >= Math.abs(dy)) {
            boolean left = dx < 0;
            input.key(left != inverted ? "right" : "left");
        } else {
            boolean up = dy < 0;
            input.wheel(up != inverted ? -360 : 360, false);
        }
        return true;
    }

    private void sendMove(float x, float y) {
        input.move(normalizedX(x), normalizedY(y));
    }

    private void sendButton(String action, float x, float y) {
        input.button(action, normalizedX(x), normalizedY(y));
    }

    private double normalizedX(float x) {
        return clamp(x / Math.max(1, surface.getWidth()));
    }

    private double normalizedY(float y) {
        return clamp(y / Math.max(1, surface.getHeight()));
    }

    private static float centerX(MotionEvent event) {
        int count = Math.min(2, event.getPointerCount());
        return count == 1 ? event.getX(0) : (event.getX(0) + event.getX(1)) / 2f;
    }

    private static float centerY(MotionEvent event) {
        int count = Math.min(2, event.getPointerCount());
        return count == 1 ? event.getY(0) : (event.getY(0) + event.getY(1)) / 2f;
    }

    private void reset() {
        state = State.IDLE;
        primaryPointer = -1;
    }

    private static double clamp(double value) {
        return Math.max(0.0, Math.min(1.0, value));
    }
}
