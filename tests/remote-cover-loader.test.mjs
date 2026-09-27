import assert from "node:assert/strict";
import test from "node:test";
import { setTimeout as delay } from "node:timers/promises";

import { createApiClient } from "../src-tauri/remote-ui/api.js";
import { CoverLoader } from "../src-tauri/remote-ui/cover-loader.js";

const detachedImage = { isConnected: false };

test("API errors preserve HTTP status for retry decisions", async () => {
  const originalFetch = globalThis.fetch;
  globalThis.fetch = async () =>
    new Response(JSON.stringify({ message: "등록된 표지가 없습니다." }), {
      status: 404,
      headers: { "Content-Type": "application/json" },
    });

  try {
    await assert.rejects(
      createApiClient("token")("/api/thumbnail"),
      (error) => {
        assert.equal(error.status, 404);
        return true;
      },
    );
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test("cover loader caches confirmed missing covers", async () => {
  let calls = 0;
  const loader = new CoverLoader(async () => {
    calls += 1;
    const error = new Error("missing");
    error.status = 404;
    throw error;
  });

  loader.load(detachedImage, "missing-item");
  await delay(10);
  loader.load(detachedImage, "missing-item");
  await delay(10);

  assert.equal(calls, 1);
});

test("cover loader retries transient failures", async () => {
  let calls = 0;
  const loader = new CoverLoader(async () => {
    calls += 1;
    const error = new Error("temporary failure");
    error.status = 503;
    throw error;
  });

  loader.load(detachedImage, "retry-item");
  await delay(10);
  loader.load(detachedImage, "retry-item");
  await delay(10);

  assert.equal(calls, 2);
});
