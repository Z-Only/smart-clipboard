use std::collections::HashMap;
use std::collections::HashSet;

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

/// N-gram based similarity engine using character-level tokenization.
pub struct NgramSimilarityEngine {
    ngram_size: usize,
}

impl NgramSimilarityEngine {
    pub fn new() -> Self {
        Self { ngram_size: 3 }
    }

    pub fn with_ngram_size(ngram_size: usize) -> Self {
        Self { ngram_size }
    }

    fn extract_ngrams(&self, text: &str) -> HashSet<String> {
        let chars: Vec<char> = text.chars().collect();
        if chars.len() < self.ngram_size {
            // For very short texts, use individual characters
            return chars.iter().map(|c| c.to_string()).collect();
        }

        let mut ngrams = HashSet::new();
        for i in 0..=chars.len() - self.ngram_size {
            let ngram: String = chars[i..i + self.ngram_size].iter().collect();
            ngrams.insert(ngram);
        }
        ngrams
    }

    fn jaccard_similarity(set_a: &HashSet<String>, set_b: &HashSet<String>) -> f64 {
        if set_a.is_empty() && set_b.is_empty() {
            return 0.0;
        }
        if set_a.is_empty() || set_b.is_empty() {
            return 0.0;
        }

        let intersection = set_a.intersection(set_b).count() as f64;
        let union = set_a.union(set_b).count() as f64;

        if union == 0.0 {
            0.0
        } else {
            intersection / union
        }
    }
}

impl Default for NgramSimilarityEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SimilarityScorer for NgramSimilarityEngine {
    fn score(&self, content_a: &str, content_b: &str) -> f64 {
        if content_a == content_b {
            return 1.0;
        }
        if content_a.is_empty() || content_b.is_empty() {
            return 0.0;
        }

        let set_a = self.extract_ngrams(content_a);
        let set_b = self.extract_ngrams(content_b);
        Self::jaccard_similarity(&set_a, &set_b)
    }

    fn score_batch(&self, query: &str, candidates: &[&str]) -> Vec<(usize, f64)> {
        let mut scores: Vec<(usize, f64)> = candidates
            .iter()
            .enumerate()
            .map(|(idx, candidate)| (idx, self.score(query, candidate)))
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores
    }

    fn engine_name(&self) -> &str {
        "NgramSimilarityEngine"
    }
}

/// Lightweight in-memory TF-IDF index for a small corpus.
pub struct TfIdfIndex {
    documents: Vec<HashMap<String, f64>>, // term → tf-idf weight per doc
    idf: HashMap<String, f64>,            // term → idf
}

impl TfIdfIndex {
    pub fn build(contents: &[&str]) -> Self {
        let n_docs = contents.len();
        let mut doc_term_freqs: Vec<HashMap<String, f64>> = Vec::with_capacity(n_docs);
        let mut doc_freq: HashMap<String, usize> = HashMap::new();

        // Compute term frequency for each document
        for content in contents {
            let tokens = tokenize(content);
            let mut term_freq: HashMap<String, f64> = HashMap::new();
            let total_terms = tokens.len() as f64;

            for token in &tokens {
                *term_freq.entry(token.clone()).or_insert(0.0) += 1.0;
            }

            // Normalize TF
            for (_term, freq) in term_freq.iter_mut() {
                *freq /= total_terms;
            }

            // Track document frequency for IDF
            let unique_terms: HashSet<&String> = term_freq.keys().collect();
            for term in unique_terms {
                *doc_freq.entry(term.clone()).or_insert(0) += 1;
            }

            doc_term_freqs.push(term_freq);
        }

        // Compute IDF: log(N / df)
        let mut idf: HashMap<String, f64> = HashMap::new();
        for (term, df) in &doc_freq {
            *idf.entry(term.clone()).or_insert(0.0) = ((n_docs as f64) / (*df as f64)).ln();
        }

        // Compute TF-IDF for each document
        let mut documents: Vec<HashMap<String, f64>> = Vec::with_capacity(n_docs);
        for doc_tf in &doc_term_freqs {
            let mut doc_tfidf: HashMap<String, f64> = HashMap::new();
            for (term, tf) in doc_tf {
                if let Some(idf_value) = idf.get(term) {
                    doc_tfidf.insert(term.clone(), tf * idf_value);
                }
            }
            documents.push(doc_tfidf);
        }

        Self { documents, idf }
    }

    pub fn query_similarity(&self, query: &str) -> Vec<(usize, f64)> {
        let query_tokens = tokenize(query);
        let mut query_tf: HashMap<String, f64> = HashMap::new();
        let total_terms = query_tokens.len() as f64;

        for token in &query_tokens {
            *query_tf.entry(token.clone()).or_insert(0.0) += 1.0;
        }

        // Normalize query TF
        for (_term, freq) in query_tf.iter_mut() {
            *freq /= total_terms;
        }

        // Compute query TF-IDF
        let mut query_tfidf: HashMap<String, f64> = HashMap::new();
        for (term, tf) in &query_tf {
            if let Some(idf_value) = self.idf.get(term) {
                query_tfidf.insert(term.clone(), tf * idf_value);
            }
        }

        // Compute cosine similarity with each document
        let mut results: Vec<(usize, f64)> = self
            .documents
            .iter()
            .enumerate()
            .map(|(idx, doc_tfidf)| {
                let sim = cosine_similarity(&query_tfidf, doc_tfidf);
                (idx, sim)
            })
            .filter(|(_, score)| *score > 0.0)
            .collect();

        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        results
    }
}

/// Compute cosine similarity between two TF-IDF weight maps.
pub fn cosine_similarity(vec_a: &HashMap<String, f64>, vec_b: &HashMap<String, f64>) -> f64 {
    if vec_a.is_empty() || vec_b.is_empty() {
        return 0.0;
    }

    let mut dot_product = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;

    for (term, weight_a) in vec_a {
        norm_a += weight_a * weight_a;
        if let Some(weight_b) = vec_b.get(term) {
            dot_product += weight_a * weight_b;
        }
    }

    for weight_b in vec_b.values() {
        norm_b += weight_b * weight_b;
    }

    norm_a = norm_a.sqrt();
    norm_b = norm_b.sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot_product / (norm_a * norm_b)
    }
}

/// Tokenize text into terms.
/// Rules:
/// - Lowercase all text
/// - Split on whitespace and common punctuation
/// - For CJK characters (Unicode range \u4e00-\u9fff), emit each character as an individual token
/// - Filter out tokens shorter than 2 chars (for non-CJK)
/// - Filter out empty tokens
pub fn tokenize(text: &str) -> Vec<String> {
    let text_lower = text.to_lowercase();
    let mut tokens: Vec<String> = Vec::new();
    let mut current_token = String::new();

    for ch in text_lower.chars() {
        if is_cjk(ch) {
            // Flush current token if any
            if !current_token.is_empty() {
                if current_token.len() >= 2 {
                    tokens.push(current_token.clone());
                }
                current_token.clear();
            }
            // Add CJK character as individual token
            tokens.push(ch.to_string());
        } else if ch.is_alphanumeric() || ch == '\'' {
            current_token.push(ch);
        } else {
            // Separator character
            if !current_token.is_empty() {
                if current_token.len() >= 2 {
                    tokens.push(current_token.clone());
                }
                current_token.clear();
            }
        }
    }

    // Don't forget the last token
    if !current_token.is_empty() && current_token.len() >= 2 {
        tokens.push(current_token);
    }

    tokens
}

fn is_cjk(ch: char) -> bool {
    matches!(ch, '\u{4e00}'..='\u{9fff}')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize_normal_text() {
        let tokens = tokenize("Hello world");
        assert_eq!(tokens, vec!["hello", "world"]);
    }

    #[test]
    fn test_tokenize_with_punctuation() {
        let tokens = tokenize("Hello, world! How are you?");
        assert_eq!(tokens, vec!["hello", "world", "how", "are", "you"]);
    }

    #[test]
    fn test_tokenize_cjk_text() {
        let tokens = tokenize("你好世界");
        assert_eq!(tokens, vec!["你", "好", "世", "界"]);
    }

    #[test]
    fn test_tokenize_mixed_text() {
        let tokens = tokenize("Hello 世界 world");
        assert_eq!(tokens, vec!["hello", "世", "界", "world"]);
    }

    #[test]
    fn test_tokenize_empty() {
        let tokens = tokenize("");
        assert_eq!(tokens, Vec::<String>::new());
    }

    #[test]
    fn test_tokenize_short_words_filtered() {
        let tokens = tokenize("I am a test");
        assert_eq!(tokens, vec!["test"]);
    }

    #[test]
    fn test_tokenize_numbers() {
        let tokens = tokenize("Test 123 abc");
        assert_eq!(tokens, vec!["test", "123", "abc"]);
    }

    #[test]
    fn test_ngram_extract_short_text() {
        let engine = NgramSimilarityEngine::with_ngram_size(3);
        let ngrams = engine.extract_ngrams("ab");
        // For short text (< ngram_size), should use individual characters
        assert!(ngrams.contains("a"));
        assert!(ngrams.contains("b"));
        assert_eq!(ngrams.len(), 2);
    }

    #[test]
    fn test_ngram_extract_long_text() {
        let engine = NgramSimilarityEngine::with_ngram_size(3);
        let ngrams = engine.extract_ngrams("hello");
        // Should have trigrams: "hel", "ell", "llo"
        assert!(ngrams.contains("hel"));
        assert!(ngrams.contains("ell"));
        assert!(ngrams.contains("llo"));
        assert_eq!(ngrams.len(), 3);
    }

    #[test]
    fn test_ngram_extract_cjk() {
        let engine = NgramSimilarityEngine::with_ngram_size(3);
        let ngrams = engine.extract_ngrams("你好世界");
        // Should have trigrams of CJK characters
        assert!(ngrams.contains("你好世"));
        assert!(ngrams.contains("好世界"));
        assert_eq!(ngrams.len(), 2);
    }

    #[test]
    fn test_ngram_score_identical() {
        let engine = NgramSimilarityEngine::new();
        let score = engine.score("hello world", "hello world");
        assert_eq!(score, 1.0);
    }

    #[test]
    fn test_ngram_score_completely_different() {
        let engine = NgramSimilarityEngine::new();
        let score = engine.score("hello", "xyz");
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_ngram_score_partially_similar() {
        let engine = NgramSimilarityEngine::new();
        let score = engine.score("hello world", "hello there");
        assert!(score > 0.0);
        assert!(score < 1.0);
    }

    #[test]
    fn test_ngram_score_empty_strings() {
        let engine = NgramSimilarityEngine::new();
        assert_eq!(engine.score("", "hello"), 0.0);
        assert_eq!(engine.score("hello", ""), 0.0);
        assert_eq!(engine.score("", ""), 0.0);
    }

    #[test]
    fn test_ngram_score_cjk_content() {
        let engine = NgramSimilarityEngine::new();
        let score = engine.score("你好世界", "你好世界");
        assert_eq!(score, 1.0);

        let score_partial = engine.score("你好世界", "你好明天");
        assert!(score_partial > 0.0);
        assert!(score_partial < 1.0);
    }

    #[test]
    fn test_ngram_score_batch_ordering() {
        let engine = NgramSimilarityEngine::new();
        let query = "hello world";
        let candidates = vec!["goodbye", "hello there", "hello world"];

        let results = engine.score_batch(query, &candidates);

        // Should be sorted by score descending
        assert_eq!(results[0].0, 2); // "hello world" should be first (score 1.0)
        assert_eq!(results[0].1, 1.0);
        assert!(results[1].1 > results[2].1); // "hello there" should score higher than "goodbye"
    }

    #[test]
    fn test_tfidf_build_and_query() {
        let docs = vec![
            "the cat sat on the mat",
            "the dog sat on the log",
            "cats and dogs are pets",
        ];

        let index = TfIdfIndex::build(&docs);
        let results = index.query_similarity("cat");

        assert!(!results.is_empty());
        // First result should be doc 0 ("the cat sat on the mat")
        assert_eq!(results[0].0, 0);
        assert!(results[0].1 > 0.0);
    }

    #[test]
    fn test_tfidf_query_no_match() {
        let docs = vec!["hello world", "foo bar"];
        let index = TfIdfIndex::build(&docs);
        let results = index.query_similarity("xyz");

        assert!(results.is_empty());
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let mut vec_a = HashMap::new();
        vec_a.insert("a".to_string(), 1.0);

        let mut vec_b = HashMap::new();
        vec_b.insert("b".to_string(), 1.0);

        let sim = cosine_similarity(&vec_a, &vec_b);
        assert_eq!(sim, 0.0);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let mut vec = HashMap::new();
        vec.insert("a".to_string(), 1.0);
        vec.insert("b".to_string(), 2.0);

        let sim = cosine_similarity(&vec, &vec);
        assert_eq!(sim, 1.0);
    }

    #[test]
    fn test_cosine_similarity_partial_overlap() {
        let mut vec_a = HashMap::new();
        vec_a.insert("a".to_string(), 1.0);
        vec_a.insert("b".to_string(), 1.0);

        let mut vec_b = HashMap::new();
        vec_b.insert("b".to_string(), 1.0);
        vec_b.insert("c".to_string(), 1.0);

        let sim = cosine_similarity(&vec_a, &vec_b);
        assert!(sim > 0.0);
        assert!(sim < 1.0);
    }

    #[test]
    fn test_cosine_similarity_empty_vectors() {
        let vec_a = HashMap::new();
        let mut vec_b = HashMap::new();
        vec_b.insert("a".to_string(), 1.0);

        let sim = cosine_similarity(&vec_a, &vec_b);
        assert_eq!(sim, 0.0);
    }

    #[test]
    fn test_similarity_scorer_trait() {
        let engine = NgramSimilarityEngine::new();
        assert_eq!(engine.engine_name(), "NgramSimilarityEngine");

        let score = engine.score("test", "test");
        assert_eq!(score, 1.0);

        let batch = engine.score_batch("test", &["test", "other"]);
        assert_eq!(batch.len(), 2);
        assert_eq!(batch[0].1, 1.0);
    }
}
