export interface PluginListItem {
  id: string;
  name: string;
  version: string | null;
  description: string | null;
  kind: string | null;
  handler: string | null;
  capabilities: string[];
  enabled: boolean;
  valid: boolean;
  error: string | null;
}

export interface PluginTransformAction {
  pluginId: string;
  pluginName: string;
  transformId: string;
  label: string;
}
