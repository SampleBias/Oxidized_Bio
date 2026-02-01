//! Search Module
//!
//! Provides scientific literature search capabilities using multiple APIs:
//! - Google Scholar (primary) - Academic papers and citations
//! - Google Light (secondary) - General web search for supplementary info
//!
//! Uses SerpAPI as the backend for both search engines.

pub mod serpapi;

pub use serpapi::{SerpApiClient, ScholarResult, LightResult, SearchError};
