import assert from "node:assert/strict";
import test from "node:test";

import {
  applyTagSuggestion,
  createSearchIndex,
  filterItems,
  getTagSuggestions,
  parseSearchTerms,
} from "../src/scripts/search.ts";
import { buildTagCatalog } from "../src/scripts/tags.ts";

function item(title, tags) {
  return {
    id: title,
    title,
    path: `C:\\library\\${title}`,
    thumbnailPath: null,
    itemType: "video",
    fileCount: 1,
    favorite: false,
    rating: null,
    notes: "",
    customThumbnailPath: null,
    tags,
    openCount: 0,
    lastOpenedAt: null,
    missing: false,
    videoFiles: [],
  };
}

const items = [
  item("both", ["topic:Travel", "genre:Travel", "ETC:Sports"]),
  item("topic-only", ["topic:Travel", "topic:Nature"]),
  item("other", ["genre:Adventure", "ETC:Outdoor"]),
];

test("comma-separated tag terms use AND semantics", () => {
  const result = filterItems(
    items,
    "topic:Travel, genre:Travel",
    createSearchIndex(items),
  );
  assert.deepEqual(
    result.map(({ title }) => title),
    ["both"],
  );
});

test("tag matching is case-insensitive and supports partial input", () => {
  assert.deepEqual(
    filterItems(items, "TOPIC:TRAV").map(({ title }) => title),
    ["both", "topic-only"],
  );
});

test("free-text terms and tag terms can be combined", () => {
  assert.deepEqual(
    filterItems(items, "both, genre:Travel").map(({ title }) => title),
    ["both"],
  );
});

test("query parser trims and removes duplicate terms", () => {
  assert.deepEqual(
    parseSearchTerms(" topic:Travel, TOPIC:travel; genre:Travel "),
    ["topic:travel", "genre:travel"],
  );
});

test("autocomplete ranks prefixes and typo-related tags", () => {
  const catalog = buildTagCatalog(items);
  assert.equal(getTagSuggestions(catalog, "topic:T")[0]?.tag, "topic:Travel");
  assert.equal(
    getTagSuggestions(catalog, "topic:Trvel")[0]?.tag,
    "topic:Travel",
  );
});

test("choosing a suggestion replaces only the active term", () => {
  assert.equal(
    applyTagSuggestion("topic:Travel, genre:T", "genre:Travel"),
    "topic:Travel, genre:Travel, ",
  );
});
