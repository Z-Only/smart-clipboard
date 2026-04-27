import { defineStore } from 'pinia';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import type { Cluster, ClipboardEntry, TagSuggestion, RelatedEntry } from '@/types';

export const useSmartStore = defineStore('smart', () => {
  const clusters = ref<Cluster[]>([]);
  const tagSuggestions = ref<Map<number, TagSuggestion[]>>(new Map());
  const relatedEntries = ref<Map<number, RelatedEntry[]>>(new Map());
  const isReclustering = ref(false);

  async function fetchClusters(): Promise<void> {
    try {
      clusters.value = await invoke<Cluster[]>('get_clusters');
    } catch (error) {
      console.error('Failed to fetch clusters:', error);
    }
  }

  async function fetchClusterEntries(
    clusterId: number,
    limit?: number,
    offset?: number,
  ): Promise<ClipboardEntry[]> {
    try {
      const result = await invoke<{ entries: ClipboardEntry[]; total_count: number }>(
        'get_cluster_entries',
        {
          clusterId,
          limit: limit ?? 50,
          offset: offset ?? 0,
        },
      );
      return result.entries;
    } catch (error) {
      console.error('Failed to fetch cluster entries:', error);
      return [];
    }
  }

  async function triggerRecluster(): Promise<void> {
    isReclustering.value = true;
    try {
      await invoke('trigger_recluster');
      await fetchClusters();
    } catch (error) {
      console.error('Failed to recluster:', error);
    } finally {
      isReclustering.value = false;
    }
  }

  async function fetchTagSuggestions(entryId: number): Promise<TagSuggestion[]> {
    try {
      const suggestions = await invoke<TagSuggestion[]>('get_tag_suggestions', { entryId });
      tagSuggestions.value.set(entryId, suggestions);
      return suggestions;
    } catch (error) {
      console.error('Failed to fetch tag suggestions:', error);
      return [];
    }
  }

  async function acceptTagSuggestion(entryId: number, tagId: number): Promise<void> {
    try {
      await invoke('accept_tag_suggestion', { entryId, tagId });
      tagSuggestions.value.delete(entryId);
    } catch (error) {
      console.error('Failed to accept tag suggestion:', error);
    }
  }

  async function dismissTagSuggestions(entryId: number): Promise<void> {
    try {
      await invoke('dismiss_tag_suggestions', { entryId });
      tagSuggestions.value.delete(entryId);
    } catch (error) {
      console.error('Failed to dismiss tag suggestions:', error);
    }
  }

  async function fetchRelatedEntries(entryId: number): Promise<RelatedEntry[]> {
    try {
      const related = await invoke<RelatedEntry[]>('get_related_entries', { entryId });
      relatedEntries.value.set(entryId, related);
      return related;
    } catch (error) {
      console.error('Failed to fetch related entries:', error);
      return [];
    }
  }

  function clearSensitiveState(): void {
    clusters.value = [];
    tagSuggestions.value.clear();
    relatedEntries.value.clear();
  }

  return {
    clusters,
    tagSuggestions,
    relatedEntries,
    isReclustering,
    fetchClusters,
    fetchClusterEntries,
    triggerRecluster,
    fetchTagSuggestions,
    acceptTagSuggestion,
    dismissTagSuggestions,
    fetchRelatedEntries,
    clearSensitiveState,
  };
});
