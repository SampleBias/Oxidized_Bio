//! SerpAPI Client
//!
//! Provides search functionality using SerpAPI for:
//! - Google Scholar: Academic papers, citations, and research
//! - Google Light: Quick general web search for supplementary information
//!
//! ## Search Strategy
//!
//! 1. **Google Scholar (Primary)**: Best for scientific/academic queries
//!    - Returns peer-reviewed papers, citations, authors
//!    - Includes DOIs and direct links to papers
//!
//! 2. **Google Light (Secondary)**: Fallback for broader searches
//!    - Faster, lighter search for general information
//!    - Useful when Scholar doesn't have enough results

use serpapi_search_rust::serp_api_search::SerpApiSearch;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use tracing::{info, warn, debug};

/// Errors that can occur during search operations
#[derive(Debug, Error)]
pub enum SearchError {
    #[error("SerpAPI key not configured")]
    NoApiKey,
    
    #[error("Search request failed: {0}")]
    RequestFailed(String),
    
    #[error("Failed to parse search results: {0}")]
    ParseError(String),
    
    #[error("No results found for query")]
    NoResults,
    
    #[error("Search engine not enabled: {0}")]
    EngineDisabled(String),
}

/// Result from a Google Scholar search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScholarResult {
    /// Title of the paper
    pub title: String,
    /// Authors of the paper
    pub authors: Option<String>,
    /// Publication year
    pub year: Option<i32>,
    /// Short snippet/abstract from the paper
    pub snippet: String,
    /// Link to the paper
    pub link: Option<String>,
    /// Citation count
    pub citations: Option<i32>,
    /// DOI if available
    pub doi: Option<String>,
    /// PDF link if available
    pub pdf_link: Option<String>,
    /// Publication venue (journal, conference, etc.)
    pub publication: Option<String>,
}

/// Result from a Google Light search
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightResult {
    /// Title of the result
    pub title: String,
    /// Snippet/description
    pub snippet: String,
    /// URL link
    pub link: String,
    /// Source domain
    pub source: Option<String>,
    /// Date if available
    pub date: Option<String>,
}

/// Combined search results from both engines
#[derive(Debug, Clone)]
pub struct CombinedSearchResults {
    /// Results from Google Scholar (primary)
    pub scholar_results: Vec<ScholarResult>,
    /// Results from Google Light (secondary)
    pub light_results: Vec<LightResult>,
    /// Whether Scholar search was successful
    pub scholar_success: bool,
    /// Whether Light search was successful
    pub light_success: bool,
    /// Any error messages
    pub errors: Vec<String>,
}

/// SerpAPI client for scientific search
pub struct SerpApiClient {
    api_key: String,
    scholar_enabled: bool,
    light_enabled: bool,
    max_results: usize,
}

impl SerpApiClient {
    /// Create a new SerpAPI client
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            scholar_enabled: true,
            light_enabled: true,
            max_results: 10,
        }
    }

    /// Configure client from config
    pub fn from_config(config: &crate::config::SearchConfig) -> Option<Self> {
        if config.serpapi_key.is_empty() {
            return None;
        }
        
        Some(Self {
            api_key: config.serpapi_key.clone(),
            scholar_enabled: config.scholar_enabled,
            light_enabled: config.light_enabled,
            max_results: config.max_results,
        })
    }

    /// Set maximum results per search
    pub fn with_max_results(mut self, max: usize) -> Self {
        self.max_results = max;
        self
    }

    /// Enable/disable Google Scholar
    pub fn with_scholar(mut self, enabled: bool) -> Self {
        self.scholar_enabled = enabled;
        self
    }

    /// Enable/disable Google Light
    pub fn with_light(mut self, enabled: bool) -> Self {
        self.light_enabled = enabled;
        self
    }

    /// Search Google Scholar for academic papers
    ///
    /// Best for scientific queries - returns peer-reviewed papers with citations
    pub async fn search_scholar(&self, query: &str) -> Result<Vec<ScholarResult>, SearchError> {
        if !self.scholar_enabled {
            return Err(SearchError::EngineDisabled("Google Scholar".to_string()));
        }

        info!(query = %query, "Searching Google Scholar via SerpAPI");

        let mut params = HashMap::<String, String>::new();
        params.insert("engine".to_string(), "google_scholar".to_string());
        params.insert("q".to_string(), query.to_string());
        params.insert("hl".to_string(), "en".to_string());
        params.insert("num".to_string(), self.max_results.to_string());

        let search = SerpApiSearch::google(params, self.api_key.clone());

        let results = search.json().await
            .map_err(|e| SearchError::RequestFailed(e.to_string()))?;

        debug!("Raw Scholar response received");

        // Parse organic results
        let organic_results = results.get("organic_results")
            .ok_or_else(|| SearchError::NoResults)?;

        let results_array = organic_results.as_array()
            .ok_or_else(|| SearchError::ParseError("Expected array of results".to_string()))?;

        if results_array.is_empty() {
            return Err(SearchError::NoResults);
        }

        let mut scholar_results = Vec::new();
        for result in results_array.iter().take(self.max_results) {
            let title = result.get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Untitled")
                .to_string();

            let snippet = result.get("snippet")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let link = result.get("link")
                .and_then(|v| v.as_str())
                .map(String::from);

            // Parse publication info
            let publication_info = result.get("publication_info");
            let authors = publication_info
                .and_then(|p| p.get("summary"))
                .and_then(|v| v.as_str())
                .map(|s| {
                    // Extract authors from summary (usually "Authors - Journal, Year")
                    s.split(" - ").next().unwrap_or(s).to_string()
                });

            // Try to extract year from publication info
            let year = publication_info
                .and_then(|p| p.get("summary"))
                .and_then(|v| v.as_str())
                .and_then(|s| {
                    // Look for a 4-digit year
                    s.split(|c: char| !c.is_numeric())
                        .find(|part| part.len() == 4)
                        .and_then(|y| y.parse::<i32>().ok())
                        .filter(|&y| y >= 1900 && y <= 2030)
                });

            let publication = publication_info
                .and_then(|p| p.get("summary"))
                .and_then(|v| v.as_str())
                .and_then(|s| {
                    // Extract journal/venue (usually after " - ")
                    let parts: Vec<&str> = s.split(" - ").collect();
                    if parts.len() > 1 {
                        Some(parts[1..].join(" - "))
                    } else {
                        None
                    }
                });

            // Get citation count
            let citations = result.get("inline_links")
                .and_then(|links| links.get("cited_by"))
                .and_then(|cited| cited.get("total"))
                .and_then(|v| v.as_i64())
                .map(|n| n as i32);

            // Get PDF link if available
            let pdf_link = result.get("resources")
                .and_then(|r| r.as_array())
                .and_then(|arr| arr.first())
                .and_then(|res| res.get("link"))
                .and_then(|v| v.as_str())
                .map(String::from);

            // Try to extract DOI from link or snippet
            let doi = link.as_ref()
                .and_then(|l| extract_doi(l))
                .or_else(|| extract_doi(&snippet));

            scholar_results.push(ScholarResult {
                title,
                authors,
                year,
                snippet,
                link,
                citations,
                doi,
                pdf_link,
                publication,
            });
        }

        info!(count = scholar_results.len(), "Google Scholar search completed");
        Ok(scholar_results)
    }

    /// Search Google Light for quick web results
    ///
    /// Faster, lighter search for general information and supplementary data
    pub async fn search_light(&self, query: &str) -> Result<Vec<LightResult>, SearchError> {
        if !self.light_enabled {
            return Err(SearchError::EngineDisabled("Google Light".to_string()));
        }

        info!(query = %query, "Searching Google Light via SerpAPI");

        let mut params = HashMap::<String, String>::new();
        params.insert("engine".to_string(), "google_light".to_string());
        params.insert("q".to_string(), query.to_string());
        params.insert("hl".to_string(), "en".to_string());
        params.insert("gl".to_string(), "us".to_string());
        params.insert("num".to_string(), self.max_results.to_string());

        let search = SerpApiSearch::google(params, self.api_key.clone());

        let results = search.json().await
            .map_err(|e| SearchError::RequestFailed(e.to_string()))?;

        debug!("Raw Light response received");

        // Parse organic results
        let organic_results = results.get("organic_results")
            .ok_or_else(|| SearchError::NoResults)?;

        let results_array = organic_results.as_array()
            .ok_or_else(|| SearchError::ParseError("Expected array of results".to_string()))?;

        if results_array.is_empty() {
            return Err(SearchError::NoResults);
        }

        let mut light_results = Vec::new();
        for result in results_array.iter().take(self.max_results) {
            let title = result.get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Untitled")
                .to_string();

            let snippet = result.get("snippet")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let link = result.get("link")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let source = result.get("source")
                .and_then(|v| v.as_str())
                .map(String::from)
                .or_else(|| {
                    // Extract domain from link
                    link.split('/').nth(2).map(String::from)
                });

            let date = result.get("date")
                .and_then(|v| v.as_str())
                .map(String::from);

            light_results.push(LightResult {
                title,
                snippet,
                link,
                source,
                date,
            });
        }

        info!(count = light_results.len(), "Google Light search completed");
        Ok(light_results)
    }

    /// Perform a combined search using both Scholar and Light
    ///
    /// Strategy:
    /// 1. Try Google Scholar first (primary - academic sources)
    /// 2. If Scholar fails or returns few results, also try Google Light
    /// 3. Combine results for comprehensive coverage
    pub async fn search_combined(&self, query: &str) -> CombinedSearchResults {
        let mut combined = CombinedSearchResults {
            scholar_results: Vec::new(),
            light_results: Vec::new(),
            scholar_success: false,
            light_success: false,
            errors: Vec::new(),
        };

        // Try Google Scholar first (primary)
        if self.scholar_enabled {
            match self.search_scholar(query).await {
                Ok(results) => {
                    combined.scholar_results = results;
                    combined.scholar_success = true;
                    info!(count = combined.scholar_results.len(), "Scholar search successful");
                }
                Err(e) => {
                    warn!(error = %e, "Scholar search failed");
                    combined.errors.push(format!("Scholar: {}", e));
                }
            }
        }

        // Try Google Light as secondary/fallback
        // Use Light if: Scholar is disabled, Scholar failed, or Scholar returned few results
        let need_light = !self.scholar_enabled 
            || !combined.scholar_success 
            || combined.scholar_results.len() < 3;

        if self.light_enabled && need_light {
            // Add scientific context to the query for better results
            let scientific_query = format!("{} research study scientific", query);
            
            match self.search_light(&scientific_query).await {
                Ok(results) => {
                    // Filter to prefer reliable sources
                    let filtered: Vec<_> = results.into_iter()
                        .filter(|r| is_reliable_source(&r.link))
                        .collect();
                    combined.light_results = filtered;
                    combined.light_success = true;
                    info!(count = combined.light_results.len(), "Light search successful");
                }
                Err(e) => {
                    warn!(error = %e, "Light search failed");
                    combined.errors.push(format!("Light: {}", e));
                }
            }
        }

        combined
    }

    /// Search with automatic engine selection based on query type
    ///
    /// - Scientific/academic queries → Scholar first, then Light
    /// - General queries → Light first, then Scholar
    pub async fn search_smart(&self, query: &str) -> CombinedSearchResults {
        // For now, always prefer Scholar for bio research context
        // Could add query classification later
        self.search_combined(query).await
    }
}

/// Extract DOI from a string (URL or text)
fn extract_doi(text: &str) -> Option<String> {
    // DOI pattern: 10.xxxx/xxxxx
    let doi_patterns = [
        "doi.org/",
        "doi:",
        "DOI:",
        "DOI ",
    ];

    for pattern in doi_patterns {
        if let Some(pos) = text.find(pattern) {
            let start = pos + pattern.len();
            let doi_part: String = text[start..]
                .chars()
                .take_while(|c| !c.is_whitespace() && *c != '"' && *c != '>' && *c != '<')
                .collect();
            if doi_part.starts_with("10.") {
                return Some(doi_part);
            }
        }
    }

    // Try to find DOI pattern directly
    if let Some(pos) = text.find("10.") {
        let doi_part: String = text[pos..]
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '.' || *c == '/' || *c == '-' || *c == '_')
            .collect();
        if doi_part.len() > 7 && doi_part.contains('/') {
            return Some(doi_part);
        }
    }

    None
}

/// Check if a URL is from a reliable scientific source
fn is_reliable_source(url: &str) -> bool {
    let reliable_domains = [
        // Academic publishers
        "pubmed.ncbi.nlm.nih.gov",
        "ncbi.nlm.nih.gov",
        "nih.gov",
        "nature.com",
        "science.org",
        "sciencedirect.com",
        "springer.com",
        "wiley.com",
        "cell.com",
        "plos.org",
        "frontiersin.org",
        "mdpi.com",
        "biomedcentral.com",
        "biorxiv.org",
        "medrxiv.org",
        "arxiv.org",
        // Academic institutions
        ".edu",
        ".ac.uk",
        // Government/research
        ".gov",
        "who.int",
        "cdc.gov",
        "fda.gov",
        "clinicaltrials.gov",
        // Databases
        "uniprot.org",
        "ensembl.org",
        "genbank",
        // Wikipedia (useful for overviews)
        "wikipedia.org",
    ];

    let url_lower = url.to_lowercase();
    reliable_domains.iter().any(|domain| url_lower.contains(domain))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_doi() {
        assert_eq!(
            extract_doi("https://doi.org/10.1234/example"),
            Some("10.1234/example".to_string())
        );
        assert_eq!(
            extract_doi("DOI: 10.5678/test-doi"),
            Some("10.5678/test-doi".to_string())
        );
        assert_eq!(
            extract_doi("no doi here"),
            None
        );
    }

    #[test]
    fn test_is_reliable_source() {
        assert!(is_reliable_source("https://pubmed.ncbi.nlm.nih.gov/12345"));
        assert!(is_reliable_source("https://www.nature.com/articles/example"));
        assert!(is_reliable_source("https://www.harvard.edu/research"));
        assert!(!is_reliable_source("https://random-blog.com/health"));
    }
}
