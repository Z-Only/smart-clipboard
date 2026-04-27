pub mod classifier;
pub mod clustering;
pub mod patterns;
pub mod sensitive;
pub mod similarity;
pub mod suggestions;

pub use classifier::{classify, Category};
pub use clustering::ClusterEngine;
pub use sensitive::detect_sensitive;
pub use similarity::{
    cosine_similarity, tokenize, NgramSimilarityEngine, SimilarityScorer, TfIdfIndex,
};
pub use suggestions::TagSuggester;
