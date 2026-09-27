import { invoke } from "@tauri-apps/api/core";
import type {
  AppPreferences,
  LibrarySnapshot,
  MetadataUpdate,
  RemoteHostStatus,
} from "./types";

export async function scanFolder(
  path: string,
): Promise<LibrarySnapshot> {
  return invoke<LibrarySnapshot>("scan_folder", { path });
}

export async function loadLibrary(
  path: string,
): Promise<LibrarySnapshot> {
  return invoke<LibrarySnapshot>("load_library", { path });
}

export async function watchLibrary(path: string): Promise<void> {
  return invoke<void>("watch_library", { path });
}

export async function getAppPreferences(): Promise<AppPreferences> {
  return invoke<AppPreferences>("get_app_preferences");
}

export async function setMinimizeToTray(
  enabled: boolean,
): Promise<AppPreferences> {
  return invoke<AppPreferences>("set_minimize_to_tray", { enabled });
}

export async function updateItemMetadata(
  rootPath: string,
  update: MetadataUpdate,
): Promise<LibrarySnapshot> {
  return invoke<LibrarySnapshot>("update_item_metadata", {
    rootPath,
    update,
  });
}

export async function recordItemOpen(
  rootPath: string,
  itemPath: string,
): Promise<LibrarySnapshot> {
  return invoke<LibrarySnapshot>("record_item_open", {
    rootPath,
    itemPath,
  });
}

export async function createUserCategory(
  rootPath: string,
  name: string,
): Promise<LibrarySnapshot> {
  return invoke<LibrarySnapshot>("create_user_category", {
    rootPath,
    name,
  });
}

export async function deleteUserCategory(
  rootPath: string,
  categoryId: number,
): Promise<LibrarySnapshot> {
  return invoke<LibrarySnapshot>("delete_user_category", {
    rootPath,
    categoryId,
  });
}

export async function setItemUserCategories(
  rootPath: string,
  itemPath: string,
  categoryIds: number[],
): Promise<LibrarySnapshot> {
  return invoke<LibrarySnapshot>("set_item_user_categories", {
    rootPath,
    itemPath,
    categoryIds,
  });
}

export async function launchItem(
  rootPath: string,
  itemPath: string,
): Promise<string> {
  return invoke<string>("launch_item", {
    rootPath,
    itemPath,
  });
}

export async function launchMediaFile(
  itemPath: string,
  mediaPath: string,
): Promise<string> {
  return invoke<string>("launch_media_file", {
    itemPath,
    mediaPath,
  });
}

export async function stopLaunchedItem(): Promise<void> {
  return invoke<void>("stop_launched_item");
}

export async function getDatabasePath(): Promise<string> {
  return invoke<string>("get_database_path");
}

export async function getRemoteHostStatus(): Promise<RemoteHostStatus> {
  return invoke<RemoteHostStatus>("get_remote_host_status");
}

export async function resetRemotePairing(): Promise<RemoteHostStatus> {
  return invoke<RemoteHostStatus>("reset_remote_pairing");
}
