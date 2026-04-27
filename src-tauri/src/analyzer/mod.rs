pub mod classifier;
pub mod patterns;
pub mod sensitive;
pub mod similarity;

pub use classifier::{classify, Category};
pub use sensitive::detect_sensitive;
pub use similarity::{
    cosine_similarity, tokenize, NgramSimilarityEngine, SimilarityScorer, TfIdfIndex,
};
