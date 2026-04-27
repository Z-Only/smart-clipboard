use super::similarity::SimilarityScorer;
use std::sync::Arc;

/// Summary of an existing cluster for assignment decisions.
pub struct ClusterSummary {
    pub id: i64,
    pub label: String,
    pub representative_content: String,
}

/// Result of clustering a set of entries.
pub struct ClusterResult {
    pub label: String,
    pub entry_ids: Vec<i64>,
}

pub struct ClusterEngine {
    scorer: Arc<dyn SimilarityScorer>,
    similarity_threshold: f64,
    max_cluster_size: usize,
}

impl ClusterEngine {
    pub fn new(
        scorer: Arc<dyn SimilarityScorer>,
        similarity_threshold: f64,
        max_cluster_size: usize,
    ) -> Self {
        Self {
            scorer,
            similarity_threshold,
            max_cluster_size,
        }
    }

    /// Try to assign an entry to an existing cluster.
    /// Returns the cluster id if similarity exceeds threshold, None otherwise.
    pub fn assign_entry(
        &self,
        entry_content: &str,
        existing_clusters: &[ClusterSummary],
    ) -> Option<i64> {
        let mut best_cluster_id: Option<i64> = None;
        let mut best_score = 0.0f64;

        for cluster in existing_clusters {
            let score = self
                .scorer
                .score(entry_content, &cluster.representative_content);
            if score > best_score {
                best_score = score;
                best_cluster_id = Some(cluster.id);
            }
        }

        if best_score >= self.similarity_threshold {
            best_cluster_id
        } else {
            None
        }
    }

    /// Run batch re-clustering on a set of entries.
    /// Returns a list of ClusterResult with labels and member entry ids.
    pub fn recluster(&self, entries: &[(i64, String)]) -> Vec<ClusterResult> {
        if entries.is_empty() {
            return Vec::new();
        }

        // For performance: if entries.len() > 1000, take a random sample of 1000
        let entries_to_cluster = if entries.len() > 1000 {
            use rand::seq::SliceRandom;
            let mut rng = rand::rng();
            let mut sampled: Vec<&(i64, String)> = entries.iter().collect();
            sampled.shuffle(&mut rng);
            sampled.truncate(1000);
            sampled
                .iter()
                .map(|&(id, ref content)| (*id, content.clone()))
                .collect::<Vec<_>>()
        } else {
            entries.to_vec()
        };

        // Simple agglomerative clustering
        // Start with each entry as its own cluster
        let mut clusters: Vec<(i64, Vec<i64>, String)> = entries_to_cluster
            .iter()
            .map(|(id, content)| (*id, vec![*id], content.clone()))
            .collect();

        loop {
            let mut best_i = 0usize;
            let mut best_j = 0usize;
            let mut best_sim = 0.0f64;
            let mut found_merge = false;

            // Find the pair of clusters with highest similarity
            for i in 0..clusters.len() {
                for j in (i + 1)..clusters.len() {
                    let sim = self.scorer.score(&clusters[i].2, &clusters[j].2);
                    if sim > best_sim {
                        best_sim = sim;
                        best_i = i;
                        best_j = j;
                        found_merge = true;
                    }
                }
            }

            // If similarity >= threshold, merge them
            if found_merge && best_sim >= self.similarity_threshold {
                // Check if merged cluster would exceed max size
                let merged_size = clusters[best_i].1.len() + clusters[best_j].1.len();
                if merged_size <= self.max_cluster_size {
                    // Merge cluster j into cluster i
                    let mut merged_ids = clusters[best_i].1.clone();
                    merged_ids.extend_from_slice(&clusters[best_j].1);
                    // Keep cluster i's representative content
                    clusters[best_i].1 = merged_ids;
                    // Remove cluster j
                    clusters.remove(best_j);
                    continue;
                }
            }

            // No more merges possible
            break;
        }

        // Generate labels for each cluster
        clusters
            .into_iter()
            .map(|(_id, entry_ids, representative)| {
                // Get contents for label generation (we only have representative here)
                let label = self.generate_label(&[&representative], "General");
                ClusterResult { label, entry_ids }
            })
            .collect()
    }

    /// Generate a label for a cluster from its member contents and category.
    pub fn generate_label(&self, entry_contents: &[&str], category: &str) -> String {
        use super::similarity::tokenize;

        // Extract the most common words (top 3) from entry_contents
        let mut word_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        for content in entry_contents {
            let tokens = tokenize(content);
            for token in tokens {
                *word_counts.entry(token).or_insert(0) += 1;
            }
        }

        // Sort by count descending and take top 3
        let mut sorted_words: Vec<(String, usize)> = word_counts.into_iter().collect();
        sorted_words.sort_by_key(|b| std::cmp::Reverse(b.1));
        sorted_words.truncate(3);

        if sorted_words.is_empty() {
            category.to_string()
        } else {
            let words_str: String = sorted_words
                .iter()
                .map(|(word, _)| word.as_str())
                .collect::<Vec<&str>>()
                .join(" ");
            format!("{}: {}", category, words_str)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::similarity::NgramSimilarityEngine;

    #[test]
    fn test_assign_entry_to_matching_cluster() {
        let engine = NgramSimilarityEngine::new();
        let cluster_engine = ClusterEngine::new(Arc::new(engine), 0.3, 50);

        let clusters = vec![
            ClusterSummary {
                id: 1,
                label: "Code".to_string(),
                representative_content: "function async import module".to_string(),
            },
            ClusterSummary {
                id: 2,
                label: "Notes".to_string(),
                representative_content: "meeting notes discussion".to_string(),
            },
        ];

        // Entry similar to first cluster
        let result = cluster_engine.assign_entry("async function import", &clusters);
        assert_eq!(result, Some(1));
    }

    #[test]
    fn test_assign_entry_returns_none_when_no_match() {
        let engine = NgramSimilarityEngine::new();
        let cluster_engine = ClusterEngine::new(Arc::new(engine), 0.8, 50);

        let clusters = vec![ClusterSummary {
            id: 1,
            label: "Code".to_string(),
            representative_content: "function async import module".to_string(),
        }];

        // Entry not similar enough
        let result = cluster_engine.assign_entry("completely different text", &clusters);
        assert_eq!(result, None);
    }

    #[test]
    fn test_recluster_basic() {
        let engine = NgramSimilarityEngine::new();
        let cluster_engine = ClusterEngine::new(Arc::new(engine), 0.3, 50);

        let entries = vec![
            (1, "hello world code".to_string()),
            (2, "hello world function".to_string()),
            (3, "completely different stuff".to_string()),
        ];

        let results = cluster_engine.recluster(&entries);
        assert!(!results.is_empty());
        // Should have at least one cluster
        assert!(results.iter().all(|r| !r.entry_ids.is_empty()));
    }

    #[test]
    fn test_generate_label() {
        let engine = NgramSimilarityEngine::new();
        let cluster_engine = ClusterEngine::new(Arc::new(engine), 0.3, 50);

        let contents = vec![
            "function async import module code",
            "async function javascript typescript",
        ];

        let label = cluster_engine.generate_label(&contents, "Code");
        assert!(label.starts_with("Code:"));
        // Should contain some of the common words
        assert!(label.contains("function") || label.contains("async") || label.contains("code"));
    }
}
