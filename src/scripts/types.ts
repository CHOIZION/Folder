export type ItemType =
  | "game"
  | "video"
  | "image"
  | "document"
  | "archive"
  | "folder";

export interface ScannedItem {
  id: string;
  title: string;
  path: string;
  thumbnailPath: string | null;
  itemType: ItemType;
  fileCount: number;
  favorite: boolean;
  rating: number | null;
  notes: string;
  customThumbnailPath: string | null;
  tags: string[];
  openCount: number;
  lastOpenedAt: string | null;
  missing: boolean;
  videoFiles: string[];
}

export interface Category {
  id: string;
  name: string;
  path: string;
  children: Category[];
  items: ScannedItem[];
}

export interface UserCategory {
  id: number;
  name: string;
  itemPaths: string[];
}

export interface LibrarySnapshot {
  rootPath: string;
  categories: Category[];
  unclassifiedItems: ScannedItem[];
  totalItems: number;
  userCategories: UserCategory[];
}

export interface MetadataUpdate {
  path: string;
  favorite: boolean;
  rating: number | null;
  notes: string;
  customThumbnailPath: string | null;
  tags: string[];
}

export interface AppPreferences {
  minimizeToTray: boolean;
}

export interface LibraryUpdatedEvent {
  rootPath: string;
  totalItems: number;
}

export interface RemoteHostStatus {
  address: string;
  port: number;
  connectionUrl: string;
  tailscaleConnectionUrl: string | null;
  pairingCode: string;
  pairedDevice: string | null;
  connected: boolean;
  lastSeenAt: number | null;
  h264Encoder: string | null;
  h264HardwareAccelerated: boolean | null;
  h264Error: string | null;
  h264Width: number | null;
  h264Height: number | null;
  errorMessage: string | null;
}
