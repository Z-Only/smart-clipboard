use std::collections::{HashMap, HashSet};

/// Trait for content similarity scoring.
/// Phase A: implemented by `NgramSimilarityEngine`.
/// Phase B: a plugin could provide a `VectorSimilarityEngine` wrapper.
pub trait SimilarityScorer: Send + Sync {
    /// Score similarity between two content strings. Returns 0.0..=1.0.
    fn score(&self, content_a: &str, content_b: &str) -> f64;

    /// Score similarity of `query` against a batch of candidates.
    /// Returns Vec<(candidate_index, score)> sorted by score descending.
    fn score_batch(&self, query: &str, candidates: &[&str]) -> Vec<(usize, f64)>;

    /// Return the engine name for logging/diagnostics.
    fn engine_name(&self) -> &str;
}

/// Character-level n-gram similarity engine using Jaccard similarity.
pub struct NgramSimilarityEngine {
    ngram_size: usize,
}

impl NgramSimilarityEngine {
    pub fn new() -> Self {
        Self { ngram_size: 3 }
    }

    pub fn with_ngram_size(ngram_size: usize) -> Self {
        Self {
            ngram_size: ngram_size.max(2),
        }
    }

    /// Extract character-level n-grams from text.
    /// For short texts (< 50 chars), uses bigrams regardless of configured size.
    fn extract_ngrams(&self, text: &str) -> HashSet<String> {
        let normalized = text.to_lowercase();
        let chars: Vec<char> = normalized.chars().collect();

        if chars.is_empty() {
            return HashSet::new();
        }

        let effective_size = if chars.len() < 50 { 2 } else { self.ngram_size };

        let mut ngrams = HashSet::new();

        // Add individual CJK characters as tokens
        for &character in &chars {
            if is_cjk(character) {
                ngrams.insert(character.to_string());
            }
        }

        if chars.len() < effective_size {
            ngrams.insert(normalized);
            return ngrams;
        }

        for window in chars.windows(effective_size) {
            let ngram: String = window.iter().collect();
            ngrams.insert(ngram);
        }

        ngrams
    }

    fn jaccard_similarity(set_a: &HashSet<String>, set_b: &HashSet<String>) -> f64 {
        if set_a.is_empty() && set_b.is_empty() {
            return 0.0;
        }
        let intersection = set_a.intersection(set_b).count() as f64;
        let union = set_a.union(set_b).count() as f64;
        if union == 0.0 {
            return 0.0;
        }
        intersection / union
    }
}

impl Default for NgramSimilarityEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SimilarityScorer for NgramSimilarityEngine {
    fn score(&self, content_a: &str, content_b: &str) -> f64 {
        if content_a.is_empty() || content_b.is_empty() {
            return 0.0;
        }
        if content_a == content_b {
            return 1.0;
        }
        let ngrams_a = self.extract_ngrams(content_a);
        let ngrams_b = self.extract_ngrams(content_b);
        Self::jaccard_similarity(&ngrams_a, &ngrams_b)
    }

    fn score_batch(&self, query: &str, candidates: &[&str]) -> Vec<(usize, f64)> {
        if query.is_empty() {
            return Vec::new();
        }
        let query_ngrams = self.extract_ngrams(query);
        let mut results: Vec<(usize, f64)> = candidates
            .iter()
            .enumerate()
            .map(|(index, candidate)| {
                let candidate_ngrams = self.extract_ngrams(candidate);
                let similarity = Self::jaccard_similarity(&query_ngrams, &candidate_ngrams);
                (index, similarity)
            })
            .filter(|(_, similarity)| *similarity > 0.0)
            .collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results
    }

    fn engine_name(&self) -> &str {
        "ngram"
    }
}

// ---------------------------------------------------------------------------
// Tokenization
// ---------------------------------------------------------------------------

/// Check if a character is in the CJK Unified Ideographs range.
fn is_cjk(character: char) -> bool {
    ('\u{4e00}'..='\u{9fff}').contains(&character)
}

/// Tokenize text into terms for TF-IDF indexing.
///
/// Rules:
/// - Lowercase all text
/// - Split on whitespace and common punctuation
/// - CJK characters are emitted as individual tokens
/// - Non-CJK tokens shorter than 2 characters are filtered out
pub fn tokenize(text: &str) -> Vec<String> {
    let lowered = text.to_lowercase();
    let mut tokens = Vec::new();
    let mut current_word = String::new();

    for character in lowered.chars() {
        if is_cjk(character) {
            // Flush any accumulated ASCII word
            if current_word.len() >= 2 {
                tokens.push(current_word.clone());
            }
            current_word.clear();
            // Emit CJK character as its own token
            tokens.push(character.to_string());
        } else if character.is_alphanumeric() || character == '_' {
            current_word.push(character);
        } else {
            // Whitespace / punctuation → flush word
            if current_word.len() >= 2 {
                tokens.push(current_word.clone());
            }
            current_word.clear();
        }
    }

    // Flush trailing word
    if current_word.len() >= 2 {
        tokens.push(current_word);
    }

    tokens
}

// ---------------------------------------------------------------------------
// TF-IDF Index
// ---------------------------------------------------------------------------

/// Lightweight in-memory TF-IDF index for a small corpus.
pub struct TfIdfIndex {
    documents: Vec<HashMap<String, f64>>,
    idf: HashMap<String, f64>,
}

impl TfIdfIndex {
    /// Build a TF-IDF index from a slice of document contents.
    pub fn build(contents: &[&str]) -> Self {
        let document_count = contents.len() as f64;
        if contents.is_empty() {
            return Self {
                documents: Vec::new(),
                idf: HashMap::new(),
            };
        }

        // Tokenize all documents and compute term frequencies
        let doc_tokens: Vec<Vec<String>> = contents.iter().map(|c| tokenize(c)).collect();

        // Compute document frequency for each term
        let mut document_frequency: HashMap<String, usize> = HashMap::new();
        for tokens in &doc_tokens {
            let unique_terms: HashSet<&str> = tokens.iter().map(|s| s.as_str()).collect();
            for term in unique_terms {
                *document_frequency.entry(term.to_string()).or_insert(0) += 1;
            }
        }

        // Compute IDF: log(N / df)
        let idf: HashMap<String, f64> = document_frequency
            .iter()
            .map(|(term, &df)| {
                let idf_value = (document_count / df as f64).ln();
                (term.clone(), idf_value)
            })
            .collect();

        // Compute TF-IDF weights per document
        let documents: Vec<HashMap<String, f64>> = doc_tokens
            .iter()
            .map(|tokens| {
                let total_terms = tokens.len() as f64;
                if total_terms == 0.0 {
                    return HashMap::new();
                }
                let mut term_count: HashMap<String, f64> = HashMap::new();
                for token in tokens {
                    *term_count.entry(token.clone()).or_insert(0.0) += 1.0;
                }
                term_count
                    .into_iter()
                    .map(|(term, count)| {
                        let tf = count / total_terms;
                        let idf_val = idf.get(&term).copied().unwrap_or(0.0);
                        (term, tf * idf_val)
                    })
                    .collect()
            })
            .collect();

        Self { documents, idf }
    }

    /// Compute cosine similarity between a query and each document.
    /// Returns Vec<(document_index, score)> sorted by score descending, excluding zeros.
    pub fn query_similarity(&self, query: &str) -> Vec<(usize, f64)> {
        let query_tokens = tokenize(query);
        if query_tokens.is_empty() {
            return Vec::new();
        }

        // Build query TF-IDF vector
        let total_terms = query_tokens.len() as f64;
        let mut term_count: HashMap<String, f64> = HashMap::new();
        for token in &query_tokens {
            *term_count.entry(token.clone()).or_insert(0.0) += 1.0;
        }
        let query_vector: HashMap<String, f64> = term_count
            .into_iter()
            .map(|(term, count)| {
                let tf = count / total_terms;
                let idf_val = self.idf.get(&term).copied().unwrap_or(0.0);
                (term, tf * idf_val)
            })
            .collect();

        let mut results: Vec<(usize, f64)> = self
            .documents
            .iter()
            .enumerate()
            .map(|(index, doc_vector)| {
                let similarity = cosine_similarity(&query_vector, doc_vector);
                (index, similarity)
            })
            .filter(|(_, similarity)| *similarity > 0.0)
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results
    }
}

// ---------------------------------------------------------------------------
// Cosine Similarity
// ---------------------------------------------------------------------------

/// Compute cosine similarity between two sparse TF-IDF vectors.
pub fn cosine_similarity(vec_a: &HashMap<String, f64>, vec_b: &HashMap<String, f64>) -> f64 {
    if vec_a.is_empty() || vec_b.is_empty() {
        return 0.0;
    }

    let dot_product: f64 = vec_a
        .iter()
        .filter_map(|(key, &val_a)| vec_b.get(key).map(|&val_b| val_a * val_b))
        .sum();

    let magnitude_a: f64 = vec_a.values().map(|v| v * v).sum::<f64>().sqrt();
    let magnitude_b: f64 = vec_b.values().map(|v| v * v).sum::<f64>().sqrt();

    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        return 0.0;
    }

    dot_product / (magnitude_a * magnitude_b)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- tokenize tests --

    #[test]
    fn tokenize_normal_text() {
        let tokens = tokenize("Hello World");
        assert_eq!(tokens, vec!["hello", "world"]);
    }

    #[test]
    fn tokenize_cjk_text() {
        let tokens = tokenize("智能剪贴板");
        assert_eq!(tokens, vec!["智", "能", "剪", "贴", "板"]);
    }

    #[test]
    fn tokenize_mixed_text() {
        let tokens = tokenize("Hello 世界 test");
        assert_eq!(tokens, vec!["hello", "世", "界", "test"]);
    }

    #[test]
    fn tokenize_empty() {
        let tokens = tokenize("");
        assert!(tokens.is_empty());
    }

    #[test]
    fn tokenize_filters_short_tokens() {
        let tokens = tokenize("I am a big cat");
        // "I", "a" are < 2 chars, filtered out
        assert_eq!(tokens, vec!["am", "big", "cat"]);
    }

    #[test]
    fn tokenize_punctuation() {
        let tokens = tokenize("hello, world! foo-bar");
        assert_eq!(tokens, vec!["hello", "world", "foo", "bar"]);
    }

    // -- NgramSimilarityEngine tests --

    #[test]
    fn ngram_identical_strings() {
        let engine = NgramSimilarityEngine::new();
        let score = engine.score("hello world", "hello world");
        assert!((score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn ngram_empty_strings() {
        let engine = NgramSimilarityEngine::new();
        assert_eq!(engine.score("", "hello"), 0.0);
        assert_eq!(engine.score("hello", ""), 0.0);
        assert_eq!(engine.score("", ""), 0.0);
    }

    #[test]
    fn ngram_completely_different() {
        let engine = NgramSimilarityEngine::new();
        let score = engine.score("aaa", "zzz");
        assert!(score < 0.1, "Expected low score, got {}", score);
    }

    #[test]
    fn ngram_partially_similar() {
        let engine = NgramSimilarityEngine::new();
        let score = engine.score("hello world", "hello earth");
        assert!(
            score > 0.0 && score < 1.0,
            "Expected partial similarity, got {}",
            score
        );
    }

    #[test]
    fn ngram_cjk_similarity() {
        let engine = NgramSimilarityEngine::new();
        let score = engine.score("智能剪贴板", "智能手机");
        assert!(score > 0.0, "Expected some CJK similarity, got {}", score);
    }

    #[test]
    fn ngram_score_batch_ordering() {
        let engine = NgramSimilarityEngine::new();
        let candidates = vec!["hello world", "goodbye universe", "hello earth"];
        let results = engine.score_batch("hello world", &candidates);
        assert!(!results.is_empty());
        // First result should be the identical string (index 0)
        assert_eq!(results[0].0, 0);
        assert!((results[0].1 - 1.0).abs() < f64::EPSILON);
        // Scores should be descending
        for window in results.windows(2) {
            assert!(window[0].1 >= window[1].1);
        }
    }

    #[test]
    fn ngram_score_batch_empty_query() {
        let engine = NgramSimilarityEngine::new();
        let results = engine.score_batch("", &["hello", "world"]);
        assert!(results.is_empty());
    }

    #[test]
    fn ngram_engine_name() {
        let engine = NgramSimilarityEngine::new();
        assert_eq!(engine.engine_name(), "ngram");
    }

    // -- cosine_similarity tests --

    #[test]
    fn cosine_identical_vectors() {
        let mut vec_a = HashMap::new();
        vec_a.insert("hello".to_string(), 1.0);
        vec_a.insert("world".to_string(), 2.0);
        let score = cosine_similarity(&vec_a, &vec_a);
        assert!((score - 1.0).abs() < 1e-10);
    }

    #[test]
    fn cosine_orthogonal_vectors() {
        let mut vec_a = HashMap::new();
        vec_a.insert("hello".to_string(), 1.0);
        let mut vec_b = HashMap::new();
        vec_b.insert("world".to_string(), 1.0);
        let score = cosine_similarity(&vec_a, &vec_b);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn cosine_partial_overlap() {
        let mut vec_a = HashMap::new();
        vec_a.insert("hello".to_string(), 1.0);
        vec_a.insert("world".to_string(), 1.0);
        let mut vec_b = HashMap::new();
        vec_b.insert("hello".to_string(), 1.0);
        vec_b.insert("earth".to_string(), 1.0);
        let score = cosine_similarity(&vec_a, &vec_b);
        assert!(score > 0.0 && score < 1.0);
        assert!((score - 0.5).abs() < 1e-10);
    }

    #[test]
    fn cosine_empty_vectors() {
        let vec_a: HashMap<String, f64> = HashMap::new();
        let vec_b: HashMap<String, f64> = HashMap::new();
        assert_eq!(cosine_similarity(&vec_a, &vec_b), 0.0);
    }

    // -- TfIdfIndex tests --

    #[test]
    fn tfidf_basic_corpus() {
        let corpus = vec![
            "the cat sat on the mat",
            "the dog sat on the log",
            "cats and dogs are friends",
        ];
        let index = TfIdfIndex::build(&corpus);
        assert_eq!(index.documents.len(), 3);
        assert!(!index.idf.is_empty());
    }

    #[test]
    fn tfidf_query_matching() {
        let corpus = vec![
            "rust programming language",
            "python programming language",
            "cooking recipes and food",
        ];
        let index = TfIdfIndex::build(&corpus);
        let results = index.query_similarity("rust programming");
        assert!(!results.is_empty());
        // The first result should be the rust document (index 0)
        assert_eq!(results[0].0, 0);
    }

    #[test]
    fn tfidf_empty_corpus() {
        let corpus: Vec<&str> = vec![];
        let index = TfIdfIndex::build(&corpus);
        let results = index.query_similarity("anything");
        assert!(results.is_empty());
    }

    #[test]
    fn tfidf_empty_query() {
        let corpus = vec!["hello world"];
        let index = TfIdfIndex::build(&corpus);
        let results = index.query_similarity("");
        assert!(results.is_empty());
    }

    #[test]
    fn tfidf_no_match() {
        let corpus = vec!["alpha beta gamma"];
        let index = TfIdfIndex::build(&corpus);
        let results = index.query_similarity("xyz completely different");
        assert!(results.is_empty());
    }

    #[test]
    fn tfidf_cjk_corpus() {
        let corpus = vec!["智能剪贴板管理工具", "会议记录和笔记", "代码片段收藏"];
        let index = TfIdfIndex::build(&corpus);
        let results = index.query_similarity("智能工具");
        assert!(!results.is_empty());
        assert_eq!(results[0].0, 0);
    }
}
