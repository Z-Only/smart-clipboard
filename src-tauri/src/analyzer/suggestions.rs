use super::similarity::SimilarityScorer;
use std::sync::Arc;

/// A tag with its id and name.
#[derive(Debug, Clone)]
pub struct TagInfo {
    pub id: i64,
    pub name: String,
}

/// A suggested tag with confidence score.
#[derive(Debug, Clone)]
pub struct TagSuggestionResult {
    pub tag: TagInfo,
    pub confidence: f64,
}

pub struct TagSuggester {
    scorer: Arc<dyn SimilarityScorer>,
    min_confidence: f64,
    max_suggestions: usize,
}

impl TagSuggester {
    pub fn new(
        scorer: Arc<dyn SimilarityScorer>,
        min_confidence: f64,
        max_suggestions: usize,
    ) -> Self {
        Self {
            scorer,
            min_confidence,
            max_suggestions,
        }
    }

    /// Suggest tags for entry_content based on similarity to already-tagged entries.
    /// tagged_entries: list of (content, tags) pairs from entries that already have tags.
    pub fn suggest_tags(
        &self,
        entry_content: &str,
        tagged_entries: &[(String, Vec<TagInfo>)],
    ) -> Vec<TagSuggestionResult> {
        if entry_content.is_empty() || tagged_entries.is_empty() {
            return Vec::new();
        }

        // For each tagged entry, compute similarity score
        // For each tag found in similar entries, accumulate a confidence score
        let mut tag_scores: std::collections::HashMap<String, (f64, usize)> =
            std::collections::HashMap::new();
        let mut tag_info_map: std::collections::HashMap<String, TagInfo> =
            std::collections::HashMap::new();

        for (content, tags) in tagged_entries {
            let similarity = self.scorer.score(entry_content, content);

            if similarity > 0.0 {
                for tag in tags {
                    let entry = tag_scores.entry(tag.name.clone()).or_insert((0.0, 0));
                    entry.0 += similarity;
                    entry.1 += 1;
                    tag_info_map.insert(tag.name.clone(), tag.clone());
                }
            }
        }

        // Calculate average confidence for each tag
        let mut results: Vec<TagSuggestionResult> = tag_scores
            .into_iter()
            .filter_map(|(tag_name, (total_score, count))| {
                let confidence = total_score / count as f64;
                if confidence >= self.min_confidence {
                    tag_info_map
                        .get(&tag_name)
                        .map(|tag_info| TagSuggestionResult {
                            tag: tag_info.clone(),
                            confidence,
                        })
                } else {
                    None
                }
            })
            .collect();

        // Sort by confidence descending
        results.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Return top max_suggestions tags
        results.truncate(self.max_suggestions);
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::similarity::NgramSimilarityEngine;

    #[test]
    fn test_suggest_tags_with_matching_content() {
        let engine = NgramSimilarityEngine::new();
        let suggester = TagSuggester::new(Arc::new(engine), 0.25, 3);

        let tagged_entries = vec![
            (
                "function async import module code".to_string(),
                vec![
                    TagInfo {
                        id: 1,
                        name: "javascript".to_string(),
                    },
                    TagInfo {
                        id: 2,
                        name: "async".to_string(),
                    },
                ],
            ),
            (
                "async function typescript".to_string(),
                vec![
                    TagInfo {
                        id: 3,
                        name: "typescript".to_string(),
                    },
                    TagInfo {
                        id: 2,
                        name: "async".to_string(),
                    },
                ],
            ),
        ];

        let results = suggester.suggest_tags("async function code", &tagged_entries);
        assert!(!results.is_empty());
        // "async" tag should have high confidence since it appears in both similar entries
        assert!(results.iter().any(|r| r.tag.name == "async"));
    }

    #[test]
    fn test_suggest_tags_below_threshold_returns_empty() {
        let engine = NgramSimilarityEngine::new();
        let suggester = TagSuggester::new(Arc::new(engine), 0.9, 3);

        let tagged_entries = vec![(
            "completely different content".to_string(),
            vec![TagInfo {
                id: 1,
                name: "unrelated".to_string(),
            }],
        )];

        let results = suggester.suggest_tags("something else entirely", &tagged_entries);
        assert!(results.is_empty());
    }

    #[test]
    fn test_suggest_tags_respects_max_suggestions() {
        let engine = NgramSimilarityEngine::new();
        let suggester = TagSuggester::new(Arc::new(engine), 0.1, 2);

        let tagged_entries = vec![
            (
                "hello world code".to_string(),
                vec![
                    TagInfo {
                        id: 1,
                        name: "tag1".to_string(),
                    },
                    TagInfo {
                        id: 2,
                        name: "tag2".to_string(),
                    },
                    TagInfo {
                        id: 3,
                        name: "tag3".to_string(),
                    },
                ],
            ),
            (
                "hello world function".to_string(),
                vec![
                    TagInfo {
                        id: 4,
                        name: "tag4".to_string(),
                    },
                    TagInfo {
                        id: 5,
                        name: "tag5".to_string(),
                    },
                ],
            ),
        ];

        let results = suggester.suggest_tags("hello world", &tagged_entries);
        assert!(results.len() <= 2);
    }
}
