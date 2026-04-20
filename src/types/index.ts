export interface Tag {
  id: number;
  name: string;
}

export interface ClipboardEntry {
  id: number;
  content: string;
  content_type: string;
  category: string;
  hash: string;
  source_app: string | null;
  is_favorite: boolean;
  is_sensitive: boolean;
  use_count: number;
  created_at: string;
  updated_at: string;
  expires_at: string | null;
}

export interface SearchResult {
  entries: ClipboardEntry[];
  total_count: number;
}

export interface Template {
  id: number | null;
  name: string;
  content: string;
  category: string;
  is_favorite: boolean;
  use_count: number;
  created_at: string;
  updated_at: string;
}

export type CategoryType =
  | "all"
  | "favorites"
  | "tags"
  | "templates"
  | "image"
  | "url"
  | "email"
  | "color"
  | "filepath"
  | "json"
  | "xml"
  | "code"
  | "phone"
  | "address"
  | "text";

export interface CategoryItem {
  key: CategoryType;
  labelKey: string;
  icon: string;
}

export const CATEGORIES: CategoryItem[] = [
  { key: "all", labelKey: "categories.all", icon: "📋" },
  { key: "favorites", labelKey: "categories.favorites", icon: "⭐" },
  { key: "tags", labelKey: "categories.tags", icon: "🏷️" },
  { key: "templates", labelKey: "categories.templates", icon: "📄" },
  { key: "image", labelKey: "categories.image", icon: "📷" },
  { key: "url", labelKey: "categories.url", icon: "🔗" },
  { key: "email", labelKey: "categories.email", icon: "📧" },
  { key: "code", labelKey: "categories.code", icon: "💻" },
  { key: "json", labelKey: "categories.json", icon: "{}" },
  { key: "filepath", labelKey: "categories.filepath", icon: "📁" },
  { key: "color", labelKey: "categories.color", icon: "🎨" },
  { key: "phone", labelKey: "categories.phone", icon: "📞" },
  { key: "address", labelKey: "categories.address", icon: "📍" },
  { key: "text", labelKey: "categories.text", icon: "📝" },
];


export type SyncStatus = "idle" | "discovering" | "online" | "offline" | "pairing" | "error" | "unknown";

export interface SyncDevice {
  id: string;
  name: string;
  deviceName: string;
  address: string | null;
  ip: string | null;
  port: number | null;
  status: SyncStatus;
  syncEnabled: boolean;
  enabled: boolean;
  lastSeenAt: string | null;
  pairedAt: string | null;
  fingerprint: string | null;
}

export interface SyncConfig {
  enabled: boolean;
  deviceName: string;
  port: number;
}

export interface UpdateSyncConfigPayload {
  enabled: boolean;
  deviceName: string;
  port: number;
}

export interface SyncStatusResponse extends SyncConfig {
  status: SyncStatus;
  pairedDevices: SyncDevice[];
  discoveredDevices: SyncDevice[];
}
