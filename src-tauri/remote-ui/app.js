import {
  applyTagSuggestion,
  buildTagCatalog,
  createSearchIndex,
  filterItemsByQuery,
  getTagSuggestions,
} from "/remote/search.js";
import { createApiClient } from "/remote/api.js";
import { CoverLoader } from "/remote/cover-loader.js";
import { $, $$, escapeHtml } from "/remote/dom.js";
const params = new URLSearchParams(location.hash.replace(/^#/, ""));
const token = params.get("token") || "";
const api = createApiClient(token);
const coverLoader = new CoverLoader(api);
const state = {
  snapshot: null,
  items: [],
  searchIndex: new Map(),
  tagCatalog: [],
  suggestions: [],
  suggestionsOpen: false,
  activeSuggestion: 0,
  category: "all",
  query: "",
  view: "library",
  streaming: false,
  streamRetryTimer: null,
  streamRetryAttempt: 0,
  streamMetricTimer: null,
  decodedFrames: 0,
  pointers: new Map(),
  gesture: "idle",
  primaryPointer: null,
  mouseDown: false,
  pressTimer: null,
  startPoint: null,
  lastPoint: null,
  lastSwipePoint: null,
  scrollPoint: null,
  swipeInverted: localStorage.getItem("heart-swipe-inverted") === "true",
  fullscreen: false,
  toastTimer: null,
  searchTimer: null,
};
let inputQueue = Promise.resolve();
let latestPointerMove = null;
let pointerMoveInFlight = false;
let pointerMoveTimer = null;
let lastPointerMoveAt = 0;
let pendingWheel = 0;
let pendingHorizontalWheel = 0;
let wheelInFlight = false;
let wheelTimer = null;
const LONG_PRESS_MS = 280;
const SWIPE_TRIGGER_PX = 42;
function toast(message, error = false) {
  const box = $("#toast");
  box.textContent = message;
  box.className = "toast show" + (error ? " error" : "");
  clearTimeout(state.toastTimer);
  state.toastTimer = setTimeout(() => (box.className = "toast"), 2600);
}
function flattenCategory(category, trail = []) {
  const label = [...trail, category.name].join(" / ");
  let result = (category.items || []).map((item) => ({
    ...item,
    categoryId: category.id,
    categoryLabel: label,
  }));
  for (const child of category.children || [])
    result = result.concat(flattenCategory(child, [...trail, category.name]));
  return result;
}

async function boot() {
  if (!token) {
    $("#grid").innerHTML =
      '<div class="empty" style="grid-column:1/-1"><b>연결 정보가 없습니다</b>HEART Remote 앱에서 다시 연결해 주세요.</div>';
    return;
  }
  try {
    const heartbeat = await (await api("/api/heartbeat")).json();
    $("#hostLabel").textContent = heartbeat.hostName || "CONNECTED";
    const snapshot = await (await api("/api/library")).json();
    state.snapshot = snapshot;
    let items = (snapshot.unclassifiedItems || []).map((item) => ({
      ...item,
      categoryId: "unclassified",
      categoryLabel: "미분류",
    }));
    for (const category of snapshot.categories || [])
      items = items.concat(flattenCategory(category));
    state.items = items;
    state.searchIndex = createSearchIndex(items);
    state.tagCatalog = buildTagCatalog(items);
    renderChips();
    renderItems();
  } catch (error) {
    $("#grid").innerHTML =
      '<div class="empty" style="grid-column:1/-1"><b>라이브러리를 열지 못했습니다</b>' +
      escapeHtml(error.message) +
      "</div>";
    toast(error.message, true);
  }
}
function renderChips() {
  const categories = [
    ...new Map(
      state.items.map((item) => [item.categoryId, item.categoryLabel]),
    ).entries(),
  ];
  $("#chips").innerHTML =
    `<button class="chip active" data-category="all">전체</button><button class="chip" data-category="favorite">♥ 즐겨찾기</button>` +
    categories
      .map(
        ([id, label]) =>
          `<button class="chip" data-category="${escapeHtml(id)}">${escapeHtml(label)}</button>`,
      )
      .join("");
  $$("#chips .chip").forEach(
    (button) =>
      (button.onclick = () => {
        state.category = button.dataset.category;
        $$("#chips .chip").forEach((item) =>
          item.classList.toggle("active", item === button),
        );
        renderItems();
      }),
  );
}
function filteredItems() {
  const categoryItems = state.items.filter((item) => {
    if (state.category === "favorite") return item.favorite;
    if (state.category !== "all") return item.categoryId === state.category;
    return true;
  });
  return filterItemsByQuery(categoryItems, state.query, state.searchIndex);
}
function renderSuggestions() {
  const box = $("#searchSuggestions"),
    input = $("#search");
  state.suggestions = getTagSuggestions(state.tagCatalog, state.query);
  state.activeSuggestion = Math.min(
    state.activeSuggestion,
    Math.max(0, state.suggestions.length - 1),
  );
  if (!state.suggestionsOpen || !state.suggestions.length) {
    box.classList.remove("open");
    input.setAttribute("aria-expanded", "false");
    input.removeAttribute("aria-activedescendant");
    return;
  }
  box.innerHTML =
    state.suggestions
      .map(
        (suggestion, index) =>
          `<button id="remote-tag-${index}" class="suggestion${index === state.activeSuggestion ? " active" : ""}" type="button" role="option" aria-selected="${index === state.activeSuggestion}" data-suggestion="${index}"><span class="symbol ${suggestion.kind}">${suggestion.kind === "topic" ? "◎" : suggestion.kind === "genre" ? "▣" : "◇"}</span><span class="tag">${escapeHtml(suggestion.tag)}</span><small>${suggestion.match} · ${suggestion.count}개</small></button>`,
      )
      .join("") +
    '<p class="suggestion-hint">↑↓ 이동 · Enter 선택 · 쉼표로 여러 태그 AND 검색</p>';
  box.classList.add("open");
  input.setAttribute("aria-expanded", "true");
  input.setAttribute(
    "aria-activedescendant",
    `remote-tag-${state.activeSuggestion}`,
  );
  $$("#searchSuggestions [data-suggestion]").forEach((button) => {
    button.onpointerdown = (event) => event.preventDefault();
    button.onpointerenter = () =>
      setActiveSuggestion(Number(button.dataset.suggestion));
    button.onclick = () => chooseSuggestion(Number(button.dataset.suggestion));
  });
}
function setActiveSuggestion(index) {
  state.activeSuggestion = index;
  $$("#searchSuggestions [data-suggestion]").forEach((button, buttonIndex) => {
    const active = buttonIndex === index;
    button.classList.toggle("active", active);
    button.setAttribute("aria-selected", String(active));
  });
  $("#search").setAttribute("aria-activedescendant", `remote-tag-${index}`);
}
function chooseSuggestion(index) {
  const suggestion = state.suggestions[index];
  if (!suggestion) return;
  state.query = applyTagSuggestion(state.query, suggestion.tag);
  state.suggestionsOpen = false;
  const input = $("#search");
  input.value = state.query;
  renderItems();
  renderSuggestions();
  input.focus();
  requestNativeKeyboard();
}
function renderItems() {
  const all = filteredItems(),
    items = all.slice(0, 180);
  $("#libraryCount").textContent = `${all.length} 작품`;
  if (!items.length) {
    $("#grid").innerHTML =
      '<div class="empty" style="grid-column:1/-1"><b>표시할 작품이 없습니다</b>검색어나 분류를 바꿔 보세요.</div>';
    return;
  }
  $("#grid").innerHTML = items
    .map(
      (item, index) =>
        `<article class="card"><div class="cover"><div class="cover-fallback">H</div><img data-cover="${index}" alt=""><div class="favorite">${item.favorite ? "♥" : ""}</div></div><div class="meta"><div class="title">${escapeHtml(item.title)}</div><div class="sub"><span>${escapeHtml(item.itemType || "item")}</span><span>·</span><span>${item.fileCount || 0} files</span></div><button class="launch" data-launch="${index}">노트북에서 실행</button></div></article>`,
    )
    .join("");
  $$("#grid [data-launch]").forEach(
    (button) =>
      (button.onclick = () => launch(items[Number(button.dataset.launch)])),
  );
  $$("#grid [data-cover]").forEach((img) =>
    coverLoader.load(img, items[Number(img.dataset.cover)].path),
  );
}
async function launch(item) {
  try {
    toast(item.title + " 실행 중…");
    await api("/api/launch", {
      method: "POST",
      body: JSON.stringify({ itemPath: item.path }),
    });
    toast(item.title + " 실행 완료");
    setView("remote");
  } catch (error) {
    toast(error.message, true);
  }
}

function setView(view) {
  state.view = view;
  $("#libraryView").classList.toggle("active", view === "library");
  $("#remoteView").classList.toggle("active", view === "remote");
  $$("nav button").forEach((button) =>
    button.classList.toggle("active", button.dataset.view === view),
  );
  state.streaming = view === "remote";
  if (state.streaming) startScreenStream();
  else stopScreenStream();
}
function startScreenStream(force = false) {
  if (!state.streaming) return;
  const image = $("#screen");
  if (force) stopScreenStream();
  if (image.dataset.streamUrl) return;

  const streamUrl = `/api/screen/stream?token=${encodeURIComponent(token)}&v=${Date.now()}`;
  image.dataset.streamUrl = streamUrl;
  $("#screenState").textContent = "노트북 화면을 연결하는 중…";
  $("#screenState").classList.remove("hidden");
  startStreamMetrics();
  image.onload = () => {
    state.decodedFrames += 1;
    image.classList.add("ready");
    $("#screenState").classList.add("hidden");
    state.streamRetryAttempt = 0;
  };
  image.onerror = () => {
    if (!state.streaming || image.dataset.streamUrl !== streamUrl) return;
    $("#screenState").textContent = "노트북 화면 스트림을 다시 연결하는 중…";
    $("#screenState").classList.remove("hidden");
    image.dataset.streamUrl = "";
    scheduleStreamRetry();
  };
  image.src = streamUrl;
}
function stopScreenStream() {
  const image = $("#screen");
  clearTimeout(state.streamRetryTimer);
  state.streamRetryTimer = null;
  stopStreamMetrics();
  image.removeAttribute("src");
  image.dataset.streamUrl = "";
  image.classList.remove("ready");
}
function scheduleStreamRetry() {
  clearTimeout(state.streamRetryTimer);
  const delay = Math.min(4_000, 350 * 2 ** state.streamRetryAttempt++);
  state.streamRetryTimer = setTimeout(() => startScreenStream(), delay);
}
function startStreamMetrics() {
  stopStreamMetrics();
  state.decodedFrames = 0;
  $("#streamMetrics").textContent = "수신 대기 중";
  state.streamMetricTimer = setInterval(() => {
    const frames = state.decodedFrames;
    state.decodedFrames = 0;
    $("#streamMetrics").textContent = `${frames} fps · 표시`;
  }, 1_000);
}
function stopStreamMetrics() {
  clearInterval(state.streamMetricTimer);
  state.streamMetricTimer = null;
  $("#streamMetrics").textContent = "스트림 일시 정지";
}
function normalized(event) {
  return normalizedPoint(event.clientX, event.clientY);
}
function normalizedPoint(clientX, clientY) {
  const image = $("#screen"),
    rect = image.getBoundingClientRect(),
    naturalWidth = image.naturalWidth || rect.width,
    naturalHeight = image.naturalHeight || rect.height,
    imageRatio = naturalWidth / naturalHeight,
    boxRatio = rect.width / rect.height;
  let left = rect.left,
    top = rect.top,
    width = rect.width,
    height = rect.height;
  if (boxRatio > imageRatio) {
    width = rect.height * imageRatio;
    left += (rect.width - width) / 2;
  } else {
    height = rect.width / imageRatio;
    top += (rect.height - height) / 2;
  }
  return {
    x: Math.max(0, Math.min(1, (clientX - left) / width)),
    y: Math.max(0, Math.min(1, (clientY - top) / height)),
  };
}
function postInput(payload) {
  return api("/api/input", { method: "POST", body: JSON.stringify(payload) });
}
function fireInput(payload) {
  inputQueue = inputQueue
    .then(() => postInput(payload))
    .catch(() => {});
  return inputQueue;
}
function queuePointerMove(point) {
  latestPointerMove = { action: "move", ...normalizedPoint(point.x, point.y) };
  if (pointerMoveInFlight || pointerMoveTimer !== null) return;
  const delay = Math.max(0, 16 - (performance.now() - lastPointerMoveAt));
  pointerMoveTimer = setTimeout(flushPointerMove, delay);
}
function flushPointerMove() {
  pointerMoveTimer = null;
  if (pointerMoveInFlight || !latestPointerMove) return;
  const payload = latestPointerMove;
  latestPointerMove = null;
  pointerMoveInFlight = true;
  lastPointerMoveAt = performance.now();
  postInput(payload)
    .catch(() => {})
    .finally(() => {
      pointerMoveInFlight = false;
      if (latestPointerMove) flushPointerMove();
    });
}
function queueWheel(delta, horizontal = false) {
  if (horizontal) pendingHorizontalWheel = Math.max(-1_200, Math.min(1_200, pendingHorizontalWheel + delta));
  else pendingWheel = Math.max(-1_200, Math.min(1_200, pendingWheel + delta));
  if (wheelInFlight || wheelTimer !== null) return;
  wheelTimer = setTimeout(flushWheel, 24);
}
function flushWheel() {
  wheelTimer = null;
  if (wheelInFlight) return;
  const horizontal = pendingHorizontalWheel !== 0;
  const delta = horizontal ? pendingHorizontalWheel : pendingWheel;
  if (!delta) return;
  if (horizontal) pendingHorizontalWheel = 0;
  else pendingWheel = 0;
  wheelInFlight = true;
  postInput({ action: horizontal ? "horizontalWheel" : "wheel", delta })
    .catch(() => {})
    .finally(() => {
      wheelInFlight = false;
      if (pendingWheel || pendingHorizontalWheel) flushWheel();
    });
}
function setMirrorFullscreen(active, notifyNative = true) {
  state.fullscreen = active;
  document.body.classList.toggle("mirror-fullscreen", active);
  if (!notifyNative) return;
  if (window.HeartAndroid) {
    if (active) {
      // The native H.264 viewer owns the desktop capture session while it is
      // visible. Closing the MJPEG request first prevents the two pipelines
      // from competing for frames and bandwidth.
      stopScreenStream();
      window.HeartAndroid.enterFullscreen(state.swipeInverted);
    } else {
      window.HeartAndroid.exitFullscreen();
      if (state.streaming) startScreenStream();
    }
    return;
  }
  if (active) {
    document.documentElement.requestFullscreen?.().catch(() => {});
    window.screen.orientation?.lock?.("landscape").catch(() => {});
  } else {
    if (document.fullscreenElement) document.exitFullscreen?.().catch(() => {});
    window.screen.orientation?.unlock?.();
  }
}
window.heartExitFullscreenFromAndroid = () => {
  setMirrorFullscreen(false, false);
  if (state.streaming) startScreenStream();
};
window.heartH264Fallback = (message) => {
  state.fullscreen = true;
  document.body.classList.add("mirror-fullscreen");
  if (state.streaming) startScreenStream(true);
  toast(`H.264를 사용할 수 없어 호환 스트림으로 전환했습니다. ${message}`, true);
};
function averagePointer() {
  const points = [...state.pointers.values()].slice(0, 2);
  return (
    {
      x: points.reduce((sum, point) => sum + point.x, 0) / Math.max(1, points.length),
      y: points.reduce((sum, point) => sum + point.y, 0) / Math.max(1, points.length),
    }
  );
}
function clearPressTimer() {
  if (state.pressTimer !== null) {
    clearTimeout(state.pressTimer);
    state.pressTimer = null;
  }
}
function activateMouseDown() {
  if (state.gesture !== "pending" || state.mouseDown || state.pointers.size !== 1)
    return;
  clearPressTimer();
  state.gesture = "mouse";
  state.mouseDown = true;
  queuePointerMove(state.lastPoint);
  fireInput({
    action: "down",
    button: "left",
    ...normalizedPoint(state.lastPoint.x, state.lastPoint.y),
  });
}
function emitSwipe(from, to) {
  const dx = to.x - from.x;
  const dy = to.y - from.y;
  if (Math.max(Math.abs(dx), Math.abs(dy)) < SWIPE_TRIGGER_PX) return false;
  if (Math.abs(dx) >= Math.abs(dy)) {
    const left = dx < 0;
    // The default follows the requested screen-pan convention: moving the
    // phone finger left advances focus to the right. The toggle swaps it.
    const key = left !== state.swipeInverted ? "right" : "left";
    fireInput({ action: "key", key });
  } else {
    const up = dy < 0;
    // Smartphone-style vertical swipes move the remote content with the finger.
    const direction = up !== state.swipeInverted ? -360 : 360;
    queueWheel(direction);
  }
  return true;
}
function finishMouseButton(action, point) {
  fireInput({ action, button: "left", ...normalizedPoint(point.x, point.y) });
}
const screen = $("#screen");
screen.addEventListener("contextmenu", (event) => event.preventDefault());
screen.addEventListener("pointerdown", (event) => {
  event.preventDefault();
  screen.setPointerCapture(event.pointerId);
  const point = { x: event.clientX, y: event.clientY };
  state.pointers.set(event.pointerId, point);
  if (state.pointers.size === 1) {
    state.gesture = "pending";
    state.primaryPointer = event.pointerId;
    state.startPoint = point;
    state.lastPoint = point;
    state.lastSwipePoint = point;
    state.mouseDown = false;
    clearPressTimer();
    state.pressTimer = setTimeout(activateMouseDown, LONG_PRESS_MS);
    return;
  }
  clearPressTimer();
  if (state.mouseDown && state.lastPoint) finishMouseButton("up", state.lastPoint);
  state.mouseDown = false;
  state.primaryPointer = null;
  state.gesture = "scroll";
  state.scrollPoint = averagePointer();
});
screen.addEventListener("pointermove", (event) => {
  if (!state.pointers.has(event.pointerId)) return;
  event.preventDefault();
  const point = { x: event.clientX, y: event.clientY };
  state.pointers.set(event.pointerId, point);
  if (state.gesture === "scroll" && state.pointers.size >= 2) {
    const average = averagePointer();
    const dx = average.x - state.scrollPoint.x;
    const dy = average.y - state.scrollPoint.y;
    state.scrollPoint = average;
    if (Math.abs(dx) >= Math.abs(dy) && Math.abs(dx) > 1)
      queueWheel(Math.round(dx * 12), true);
    else if (Math.abs(dy) > 1) queueWheel(Math.round(-dy * 12));
    return;
  }
  if (event.pointerId !== state.primaryPointer)
    return;
  state.lastPoint = point;
  if (state.gesture === "mouse") {
    queuePointerMove(point);
    return;
  }
  if (state.gesture === "pending") {
    if (emitSwipe(state.startPoint, point)) {
      clearPressTimer();
      state.gesture = "swipe";
      state.lastSwipePoint = point;
    }
    return;
  }
  if (state.gesture === "swipe" && emitSwipe(state.lastSwipePoint, point))
    state.lastSwipePoint = point;
});
function finishPointer(event, cancelled = false) {
  if (!state.pointers.has(event.pointerId)) return;
  event.preventDefault();
  const wasPrimary = event.pointerId === state.primaryPointer;
  const gesture = state.gesture;
  const point = { x: event.clientX, y: event.clientY };
  state.pointers.delete(event.pointerId);
  if (wasPrimary) {
    clearPressTimer();
    if (gesture === "mouse" && state.mouseDown) finishMouseButton("up", point);
    else if (gesture === "pending" && !cancelled) finishMouseButton("click", point);
    state.mouseDown = false;
    state.primaryPointer = null;
    state.gesture = state.pointers.size ? "scrollEnding" : "idle";
    return;
  }
  if (state.gesture === "scroll" && state.pointers.size < 2)
    state.gesture = state.pointers.size ? "scrollEnding" : "idle";
  else if (state.gesture === "scrollEnding" && !state.pointers.size)
    state.gesture = "idle";
}
screen.addEventListener("pointerup", (event) => finishPointer(event, false));
screen.addEventListener("pointercancel", (event) => finishPointer(event, true));

const searchInput = $("#search");
searchInput.addEventListener("input", () => {
  state.query = searchInput.value;
  state.activeSuggestion = 0;
  state.suggestionsOpen = true;
  clearTimeout(state.searchTimer);
  state.searchTimer = setTimeout(renderItems, 80);
  renderSuggestions();
});
searchInput.addEventListener("focus", () => {
  state.suggestionsOpen = true;
  renderSuggestions();
  requestNativeKeyboard();
});
searchInput.addEventListener("blur", () => {
  state.suggestionsOpen = false;
  setTimeout(renderSuggestions, 0);
});
searchInput.addEventListener("keydown", (event) => {
  if (event.key === "Escape") {
    state.suggestionsOpen = false;
    renderSuggestions();
    return;
  }
  if (!state.suggestions.length) return;
  if (event.key === "ArrowDown" || event.key === "ArrowUp") {
    event.preventDefault();
    const direction = event.key === "ArrowDown" ? 1 : -1;
    setActiveSuggestion(
      (state.activeSuggestion + direction + state.suggestions.length) %
        state.suggestions.length,
    );
    return;
  }
  if (event.key === "Enter" || event.key === "Tab") {
    event.preventDefault();
    chooseSuggestion(state.activeSuggestion);
  }
});
$$("nav button").forEach(
  (button) => (button.onclick = () => setView(button.dataset.view)),
);
$$(".key[data-key]").forEach(
  (button) =>
    (button.onclick = () =>
      fireInput({ action: "key", key: button.dataset.key })),
);
$$(".key[data-action]").forEach(
  (button) =>
    (button.onclick = () => {
      const action = button.dataset.action;
      if (action === "right") fireInput({ action: "click", button: "right" });
      if (action === "scrollUp") fireInput({ action: "wheel", delta: 360 });
      if (action === "scrollDown") fireInput({ action: "wheel", delta: -360 });
      if (action === "keyboard") {
        const modal = $("#textModal");
        modal.classList.add("open");
        const input = $("#textInput");
        input.focus();
        requestNativeKeyboard();
      }
    }),
);
$("#showHeart").onclick = async () => {
  try {
    await api("/api/show-heart", { method: "POST" });
    toast("HEART를 앞으로 가져왔습니다.");
  } catch (error) {
    toast(error.message, true);
  }
};
$("#stopItem").onclick = async () => {
  try {
    await api("/api/stop", { method: "POST" });
    toast("실행 중인 작품을 종료했습니다.");
  } catch (error) {
    toast(error.message, true);
  }
};
$("#refreshScreen").onclick = () => {
  startScreenStream(true);
};
$("#toggleSwipeDirection").onclick = () => {
  state.swipeInverted = !state.swipeInverted;
  localStorage.setItem("heart-swipe-inverted", String(state.swipeInverted));
  updateSwipeDirectionLabel();
  toast(`스와이프 방향을 ${state.swipeInverted ? "반전" : "기본"}으로 바꿨습니다.`);
};
$("#enterFullscreen").onclick = () => setMirrorFullscreen(true);
$("#exitFullscreen").onclick = () => setMirrorFullscreen(false);
$("#cancelText").onclick = () => $("#textModal").classList.remove("open");
$("#sendText").onclick = async () => {
  const input = $("#textInput"),
    text = input.value;
  if (text) {
    await fireInput({ action: "text", text });
    input.value = "";
    toast("문자를 전송했습니다.");
  }
  $("#textModal").classList.remove("open");
};
$("#textInput").addEventListener("keydown", (event) => {
  if (event.key === "Enter") $("#sendText").click();
});
function requestNativeKeyboard() {
  window.HeartAndroid?.showKeyboard?.();
}
function updateSwipeDirectionLabel() {
  $("#toggleSwipeDirection").textContent = `스와이프 반전: ${state.swipeInverted ? "켬" : "끔"}`;
}
document.addEventListener("visibilitychange", () => {
  if (document.hidden) stopScreenStream();
  else if (state.view === "remote") startScreenStream();
});
window.addEventListener("beforeunload", () => {
  stopScreenStream();
  coverLoader.dispose();
});
updateSwipeDirectionLabel();
boot();
