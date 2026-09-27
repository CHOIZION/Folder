import { FAVORITES_CATEGORY_ID, RECENT_CATEGORY_ID } from "./library";
import type { LibraryController } from "./library-controller.svelte";
import type { ScannedItem } from "./types";

const HORIZONTAL_PADDING = 205;
const VERTICAL_PADDING = 215;

export class PulseController {
  open = $state(false);
  x = $state(0);
  y = $state(0);

  constructor(private readonly library: LibraryController) {}

  handleMiddleMouse(event: MouseEvent): void {
    if (event.button !== 1) return;
    event.preventDefault();
    if (this.open) {
      this.open = false;
      return;
    }
    this.x = clampCoordinate(
      event.clientX,
      window.innerWidth,
      HORIZONTAL_PADDING,
    );
    this.y = clampCoordinate(
      event.clientY,
      window.innerHeight,
      VERTICAL_PADDING,
    );
    this.open = true;
  }

  search(query: string): void {
    this.resetView(null);
    this.library.searchText = query;
  }

  selectItem(item: ScannedItem | null): void {
    if (!item) return;
    this.resetView(null);
    this.library.selectItem(item);
  }

  selectRandom(): void {
    const items = this.library.pulse.items;
    if (!items.length) return;
    this.selectItem(items[Math.floor(Math.random() * items.length)]);
  }

  showRecent(): void {
    this.resetView(RECENT_CATEGORY_ID);
  }

  showAll(): void {
    this.resetView(null);
  }

  showFavorites(): void {
    this.resetView(FAVORITES_CATEGORY_ID);
  }

  private resetView(categoryId: string | null): void {
    this.library.activeView = "library";
    this.library.selectedCategoryId = categoryId;
    this.library.selectedRating = null;
    this.library.selectedItemPath = null;
    this.library.searchText = "";
    this.library.detailPanelOpen = false;
    this.library.statisticsOpen = false;
    this.open = false;
  }
}

function clampCoordinate(value: number, size: number, padding: number): number {
  if (size <= padding * 2) return size / 2;
  return Math.min(Math.max(value, padding), size - padding);
}
