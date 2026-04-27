use std::sync::Arc;

use tauri::State;

use crate::analyzer::similarity::SimilarityScorer;
use crate::encryption::EncryptionManager;
use crate::security::AppLockManager;
use crate::storage::Database;

use super::{decrypt_search_result, require_unlocked};

// --- Cluster commands ---

#[tauri::command]
pub async fn get_clusters(
    lock: State<'_, Arc<AppLockManager>>,
    db: State<'_, Arc<Database>>,
) -> Result<Vec<crate::storage::ClusterSummary>, String> {
    require_unlocked(&lock)?;
    db.get_cluster_list().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_cluster_entries(
    lock: State<'_, Arc<AppLockManager>>,
    encryption: State<'_, Arc<EncryptionManager>>,
    db: State<'_, Arc<Database>>,
    cluster_id: i64,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<crate::storage::SearchResult, String> {
    require_unlocked(&lock)?;
    let mut result = db
        .get_cluster_entries(cluster_id, limit.unwrap_or(50), offset.unwrap_or(0))
        .map_err(|e| e.to_string())?;
    decrypt_search_result(&encryption, &mut result);
    Ok(result)
}

#[tauri::command]
pub async fn trigger_recluster(
    lock: State<'_, Arc<AppLockManager>>,
    db: State<'_, Arc<Database>>,
    config: State<'_, Arc<crate::config::ConfigManager>>,
) -> Result<(), String> {
    require_unlocked(&lock)?;
    let app_config = config.get();
    if !app_config.smart_search_enabled {
        return Ok(());
    }

    let scorer = Arc::new(crate::analyzer::NgramSimilarityEngine::new());
    let engine =
        crate::analyzer::ClusterEngine::new(scorer, app_config.cluster_similarity_threshold, 50);

    // Clear existing clusters
    db.clear_clusters().map_err(|e| e.to_string())?;

    // Get all entries for clustering
    let entries_raw = db
        .get_unclustered_entries(5000)
        .map_err(|e| e.to_string())?;
    let entries: Vec<(i64, String)> = entries_raw
        .iter()
        .filter_map(|e| e.id.map(|id| (id, e.content.clone())))
        .collect();

    let cluster_results = engine.recluster(&entries);

    for result in &cluster_results {
        let cluster_id = db
            .upsert_cluster(&result.label)
            .map_err(|e| e.to_string())?;
        for &entry_id in &result.entry_ids {
            db.add_to_cluster(cluster_id, entry_id, 1.0)
                .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

// --- Tag suggestion commands ---

#[tauri::command]
pub async fn get_tag_suggestions(
    lock: State<'_, Arc<AppLockManager>>,
    db: State<'_, Arc<Database>>,
    entry_id: i64,
) -> Result<Vec<crate::storage::TagSuggestion>, String> {
    require_unlocked(&lock)?;
    db.get_tag_suggestions(entry_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn accept_tag_suggestion(
    lock: State<'_, Arc<AppLockManager>>,
    db: State<'_, Arc<Database>>,
    entry_id: i64,
    tag_id: i64,
) -> Result<(), String> {
    require_unlocked(&lock)?;
    db.add_tag_to_entry(entry_id, tag_id)
        .map_err(|e| e.to_string())?;
    // Remove this specific suggestion
    db.dismiss_tag_suggestions(entry_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn dismiss_tag_suggestions(
    lock: State<'_, Arc<AppLockManager>>,
    db: State<'_, Arc<Database>>,
    entry_id: i64,
) -> Result<(), String> {
    require_unlocked(&lock)?;
    db.dismiss_tag_suggestions(entry_id)
        .map_err(|e| e.to_string())
}

// --- Related entries command ---

#[tauri::command]
pub async fn get_related_entries(
    lock: State<'_, Arc<AppLockManager>>,
    encryption: State<'_, Arc<EncryptionManager>>,
    db: State<'_, Arc<Database>>,
    config: State<'_, Arc<crate::config::ConfigManager>>,
    entry_id: i64,
) -> Result<Vec<crate::storage::RelatedEntry>, String> {
    require_unlocked(&lock)?;
    let app_config = config.get();
    if !app_config.smart_search_enabled {
        return Ok(Vec::new());
    }

    let source_entry = db
        .get_entry_by_id(entry_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "Entry not found".to_string())?;

    let max_related = app_config.max_related_entries as i64;
    let candidates = db
        .get_entries_for_similarity(entry_id, 150)
        .map_err(|e| e.to_string())?;

    if candidates.is_empty() {
        return Ok(Vec::new());
    }

    let scorer = crate::analyzer::NgramSimilarityEngine::new();
    let candidate_contents: Vec<&str> = candidates.iter().map(|e| e.content.as_str()).collect();
    let scores = scorer.score_batch(&source_entry.content, &candidate_contents);

    let related: Vec<crate::storage::RelatedEntry> = scores
        .into_iter()
        .take(max_related as usize)
        .filter(|(_, score)| *score > 0.05)
        .map(|(idx, score)| {
            let mut entry: crate::storage::ClipboardEntry = candidates[idx].clone();
            if let Ok(decrypted) = encryption.decrypt_content(&entry.content) {
                entry.content = decrypted;
            }
            crate::storage::RelatedEntry { entry, score }
        })
        .collect();

    Ok(related)
}
