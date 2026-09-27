import assert from "node:assert/strict";
import test from "node:test";

import {
  applyTagSuggestion,
  buildTagCatalog,
  filterItemsByQuery,
  getTagSuggestions,
} from "../src-tauri/remote-ui/search.js";

const items = [
  {
    title: "both",
    path: "both",
    itemType: "video",
    notes: "",
    tags: ["topic:Travel", "genre:Travel"],
  },
  {
    title: "topic-only",
    path: "topic",
    itemType: "video",
    notes: "",
    tags: ["topic:Travel"],
  },
  {
    title: "other",
    path: "other",
    itemType: "video",
    notes: "",
    tags: ["ETC:Outdoor"],
  },
];

test("remote search also requires every comma-separated tag", () => {
  assert.deepEqual(
    filterItemsByQuery(items, "topic:Travel, genre:Travel").map(
      ({ title }) => title,
    ),
    ["both"],
  );
});

test("remote suggestions support prefix, typo, and active-term replacement", () => {
  const catalog = buildTagCatalog(items);
  assert.equal(getTagSuggestions(catalog, "topic:T")[0]?.tag, "topic:Travel");
  assert.equal(getTagSuggestions(catalog, "topic:Trvel")[0]?.match, "연관");
  assert.equal(
    applyTagSuggestion("topic:Travel, genre:T", "genre:Travel"),
    "topic:Travel, genre:Travel, ",
  );
});
