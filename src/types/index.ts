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

export type CategoryType =
  | "all"
  | "favorites"
  | "tags"
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
