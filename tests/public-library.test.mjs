import assert from "node:assert/strict";
import test from "node:test";
import { parseVoiceIntent, findCategoryIdByVoice } from "../src/scripts/assistant.ts";
import { filterItems } from "../src/scripts/search.ts";
import { getTagKind } from "../src/scripts/tags.ts";
import { filterItemsByQuery } from "../src-tauri/remote-ui/search.js";

test("neutral voice categories resolve Korean and English folders", () => {
  for (const [spoken, expected] of [["사진", "이미지"], ["비디오", "영상"], ["문서", "문서"], ["게임", "게임"]]) {
    assert.deepEqual(parseVoiceIntent(spoken), { type: "category", categoryName: expected });
  }
  const categories = [{ id: "video", name: "Videos", children: [], items: [] }];
  assert.equal(findCategoryIdByVoice(categories, "영상"), "video");
  assert.deepEqual(parseVoiceIntent("자비스, 도입 포함 40분 세팅 부탁해"), {
    type: "session", minutes: 40, warmup: "full",
  });
});

test("custom tag namespaces search tags rather than similarly worded notes", () => {
  const common = { path: "/sample", itemType: "video", notes: "", tags: [] };
  const items = [
    { ...common, path: "/sample/tagged", title: "tagged", tags: ["creator:Example", "topic:Travel"] },
    { ...common, path: "/sample/notes", title: "notes-only", notes: "creator:Example", tags: ["topic:Travel"] },
  ];
  const query = "creator:Example, topic:Travel";
  assert.deepEqual(filterItems(items, query).map(x => x.title), ["tagged"]);
  assert.deepEqual(filterItemsByQuery(items, query).map(x => x.title), ["tagged"]);
  assert.equal(getTagKind("genre:Documentary"), "genre");
  assert.equal(getTagKind("topic:Travel"), "topic");
  assert.equal(getTagKind("creator:Example"), "etc");
});

test("Windows paths remain free-text search terms", () => {
  const items = [{ title: "Sample", path: "C:\\Media\\Sample", itemType: "video", notes: "", tags: [] }];
  assert.equal(filterItems(items, "C:\\Media").length, 1);
  assert.equal(filterItemsByQuery(items, "C:\\Media").length, 1);
});
