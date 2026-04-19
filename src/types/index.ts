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
  label: string;
  icon: string;
}

export const CATEGORIES: CategoryItem[] = [
  { key: "all", label: "All", icon: "📋" },
  { key: "favorites", label: "Favorites", icon: "⭐" },
  { key: "url", label: "Links", icon: "🔗" },
  { key: "email", label: "Email", icon: "📧" },
  { key: "code", label: "Code", icon: "💻" },
  { key: "json", label: "JSON", icon: "{}" },
  { key: "filepath", label: "Files", icon: "📁" },
  { key: "color", label: "Colors", icon: "🎨" },
  { key: "phone", label: "Phone", icon: "📞" },
  { key: "address", label: "Address", icon: "📍" },
  { key: "text", label: "Text", icon: "📝" },
];
