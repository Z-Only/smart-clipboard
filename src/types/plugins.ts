export interface PluginListItem {
  id: string;
  name: string;
  description: string | null;
  version: string;
  enabled: boolean;
}

export interface PluginTransformAction {
  plugin_id: string;
  action_id: string;
  label: string;
  description: string | null;
}
