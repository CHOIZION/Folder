import assert from "node:assert/strict";
import test from "node:test";

import {
  UNCLASSIFIED_CATEGORY_ID,
  getItemsForCategory,
  needsClassification,
} from "../src/scripts/library.ts";

function item(path, rating, tags) {
  return {
    id: path,
    title: path,
    path,
    thumbnailPath: null,
    itemType: "folder",
    fileCount: 1,
    favorite: false,
    rating,
    notes: "",
    customThumbnailPath: null,
    tags,
    openCount: 0,
    lastOpenedAt: null,
    missing: false,
    videoFiles: [],
  };
}

test("classification inbox includes items missing a rating or tags", () => {
  const complete = item("complete", 4, ["topic:Travel"]);
  const noRating = item("no-rating", null, ["topic:Travel"]);
  const noTags = item("no-tags", 3, []);
  const empty = item("empty", null, []);
  const snapshot = {
    rootPath: "C:\\HEART",
    categories: [
      {
        id: "category",
        name: "Category",
        path: "C:\\HEART\\Category",
        children: [],
        items: [complete, noRating, noTags],
      },
    ],
    unclassifiedItems: [empty],
    totalItems: 4,
    userCategories: [],
  };

  assert.equal(needsClassification(complete), false);
  assert.deepEqual(
    getItemsForCategory(snapshot, UNCLASSIFIED_CATEGORY_ID).map(
      ({ path }) => path,
    ),
    ["no-rating", "no-tags", "empty"],
  );
});
