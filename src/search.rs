//! Full-text search functionality for indexed notes.
//!
//! This module provides powerful search capabilities using SQLite's FTS5
//! (Full-Text Search) extension. It enables searching through note titles
//! and content with support for ranking, snippets, and various search options.
//!
//! # Features
//!
//! - **Full-text search**: Search across both title and content
//! - **Field-specific search**: Search only titles or only content
//! - **BM25 ranking**: Results are ranked by relevance using the BM25 algorithm
//! - **Snippets**: Extract highlighted text fragments around matches
//! - **Pagination**: Support for limit/offset pagination
//! - **Suggestions**: Auto-complete suggestions for note titles
//! - **Advanced queries**: Support for complex FTS5 query syntax
//!
//! # Usage
//!
//! ```rust,no_run
//! use ora_core::search::{Query, SearchOptions};
//! use ora_core::watcher::index::Index;
//! # use std::path::Path;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let index = Index::new(Path::new("/path/to/shelf"))?;
//! // Create a query from an index
//! let query = Query::new(&index);
//!
//! // Simple search
//! let results = query.search("rust programming")?;
//!
//! // Search with options
//! let options = SearchOptions {
//!     limit: Some(10),
//!     include_snippets: true,
//!     snippet_length: 150,
//!     ..Default::default()
//! };
//! let results = query.search_with_options("rust programming", &options)?;
//! # Ok(())
//! # }
//! ```

use crate::error::OraError;
use crate::watcher::index::{Index, IndexedNote};
use rusqlite::{Connection, params};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// A search query interface for the note index.
///
/// Provides methods for searching through indexed notes using SQLite's FTS5
/// full-text search capabilities. The query maintains a connection to the
/// search index and executes various types of searches.
pub struct Query {
    conn: Arc<Mutex<Connection>>,
}

/// A single search result containing a matched note and metadata.
///
/// Represents one note that matched a search query, along with relevance
/// information and optional text snippets showing where the match occurred.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// The note that matched the search query.
    pub note: IndexedNote,

    /// Relevance score calculated using the BM25 algorithm.
    ///
    /// Lower scores indicate better matches. The exact scale depends on
    /// the FTS5 configuration and document statistics.
    pub rank: f64,

    /// Optional highlighted snippet from the note title.
    ///
    /// Contains the matched text surrounded by `<mark>` tags when
    /// snippets are requested. `None` if snippets are disabled.
    pub snippet_title: Option<String>,

    /// Optional highlighted snippet from the note content.
    ///
    /// Contains the matched text surrounded by `<mark>` tags when
    /// snippets are requested. `None` if snippets are disabled.
    pub snippet_content: Option<String>,
}

/// Configuration options for search queries.
///
/// Controls how search results are returned, including pagination,
/// snippet generation, and result limits.
#[derive(Debug, Clone)]
pub struct SearchOptions {
    /// Maximum number of results to return.
    ///
    /// `None` means no explicit limit (database defaults may apply).
    /// Defaults to `Some(50)`.
    pub limit: Option<u32>,

    /// Number of results to skip for pagination.
    ///
    /// Used in combination with `limit` for paginated results.
    /// Defaults to `Some(0)`.
    pub offset: Option<u32>,

    /// Whether to generate text snippets around matches.
    ///
    /// When `true`, `snippet_title` and `snippet_content` fields in
    /// [`SearchResult`] will contain highlighted text fragments.
    /// Defaults to `true`.
    pub include_snippets: bool,

    /// Maximum length of generated snippets in characters.
    ///
    /// Only used when `include_snippets` is `true`.
    /// Defaults to `100`.
    pub snippet_length: u32,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            limit: Some(50),
            offset: Some(0),
            include_snippets: true,
            snippet_length: 100,
        }
    }
}

impl Query {
    /// Creates a new search query using the provided index.
    ///
    /// # Arguments
    /// * `index` - The search index to query against
    ///
    /// # Returns
    /// A new [`Query`] instance ready for searching
    pub fn new(index: &Index) -> Self {
        Self {
            conn: index.conn.clone(),
        }
    }

    /// Performs a simple search across both title and content.
    ///
    /// Uses default search options. For more control over the search,
    /// use `search_with_options`.
    ///
    /// # Arguments
    /// * `query` - The search query string (supports FTS5 syntax)
    ///
    /// # Returns
    /// A vector of search results ranked by relevance
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use ora_core::search::Query;
    /// # use ora_core::watcher::index::Index;
    /// # use std::path::Path;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let index = Index::new(Path::new("/path/to/shelf"))?;
    /// # let query = Query::new(&index);
    /// let results = query.search("rust programming")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn search(&self, query: &str) -> Result<Vec<SearchResult>, OraError> {
        self.search_with_options(query, &SearchOptions::default())
    }

    /// Performs a search with custom options.
    ///
    /// This is the main search method that all other search methods
    /// eventually call. It supports full FTS5 query syntax and provides
    /// complete control over result formatting.
    ///
    /// # Arguments
    /// * `query` - The search query string (supports FTS5 syntax)
    /// * `options` - Search configuration options
    ///
    /// # Returns
    /// A vector of search results ranked by relevance
    ///
    /// # FTS5 Query Syntax
    ///
    /// The query supports SQLite's FTS5 syntax including:
    /// - `term` - Simple term matching
    /// - `"phrase"` - Exact phrase matching
    /// - `term1 AND term2` - Boolean AND
    /// - `term1 OR term2` - Boolean OR
    /// - `term NOT term2` - Boolean NOT
    /// - `title:term` - Search only title field
    /// - `content:term` - Search only content field
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use ora_core::search::{Query, SearchOptions};
    /// # use ora_core::watcher::index::Index;
    /// # use std::path::Path;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let index = Index::new(Path::new("/path/to/shelf"))?;
    /// # let query = Query::new(&index);
    /// let options = SearchOptions {
    ///     limit: Some(10),
    ///     include_snippets: true,
    ///     ..Default::default()
    /// };
    /// let results = query.search_with_options("rust AND programming", &options)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn search_with_options(
        &self,
        query: &str,
        options: &SearchOptions,
    ) -> Result<Vec<SearchResult>, OraError> {
        let conn = self.conn.lock().unwrap();
        let limit = options.limit.unwrap_or(50);
        let offset = options.offset.unwrap_or(0);

        let sql = if options.include_snippets {
            format!(
                r#"
                SELECT 
                    n.title,
                    n.content,
                    n.path,
                    bm25(contents) as rank,
                    snippet(contents, 0, '<mark>', '</mark>', '...', {}) as title_snippet,
                    snippet(contents, 1, '<mark>', '</mark>', '...', {}) as content_snippet
                FROM contents
                JOIN notes n ON n.id = contents.rowid
                WHERE contents MATCH ?
                ORDER BY rank
                LIMIT ? OFFSET ?
                "#,
                options.snippet_length, options.snippet_length
            )
        } else {
            r#"
            SELECT 
                n.title,
                n.content,
                n.path,
                bm25(contents) as rank
            FROM contents
            JOIN notes n ON n.id = contents.rowid
            WHERE contents MATCH ?
            ORDER BY rank
            LIMIT ? OFFSET ?
            "#
            .to_string()
        };

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params![query, limit, offset], |row| {
            let title: String = row.get(0)?;
            let content: String = row.get(1)?;
            let path_str: String = row.get(2)?;
            let rank: f64 = row.get(3)?;

            let (title_snippet, content_snippet) = if options.include_snippets {
                let title_snippet: Option<String> = row.get(4).ok();
                let content_snippet: Option<String> = row.get(5).ok();
                (title_snippet, content_snippet)
            } else {
                (None, None)
            };

            Ok(SearchResult {
                note: IndexedNote {
                    title,
                    content,
                    path: PathBuf::from(path_str),
                },
                rank,
                snippet_title: title_snippet,
                snippet_content: content_snippet,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }

        Ok(results)
    }

    /// Searches only within note titles.
    ///
    /// Uses FTS5 column-specific syntax to restrict the search to the title field.
    /// Uses default search options.
    ///
    /// # Arguments
    /// * `query` - The search query string
    ///
    /// # Returns
    /// A vector of search results where the query matched in the title
    pub fn search_title(&self, query: &str) -> Result<Vec<SearchResult>, OraError> {
        self.search_title_with_options(query, &SearchOptions::default())
    }

    /// Searches only within note titles with custom options.
    ///
    /// Uses FTS5 column-specific syntax to restrict the search to the title field.
    ///
    /// # Arguments
    /// * `query` - The search query string
    /// * `options` - Search configuration options
    ///
    /// # Returns
    /// A vector of search results where the query matched in the title
    pub fn search_title_with_options(
        &self,
        query: &str,
        options: &SearchOptions,
    ) -> Result<Vec<SearchResult>, OraError> {
        // For title-only search, we use FTS5 with column-specific syntax
        let title_query = format!("title:{}", query);
        self.search_with_options(&title_query, options)
    }

    /// Searches only within note content.
    ///
    /// Uses FTS5 column-specific syntax to restrict the search to the content field.
    /// Uses default search options.
    ///
    /// # Arguments
    /// * `query` - The search query string
    ///
    /// # Returns
    /// A vector of search results where the query matched in the content
    pub fn search_content(&self, query: &str) -> Result<Vec<SearchResult>, OraError> {
        self.search_content_with_options(query, &SearchOptions::default())
    }

    /// Searches only within note content with custom options.
    ///
    /// Uses FTS5 column-specific syntax to restrict the search to the content field.
    ///
    /// # Arguments
    /// * `query` - The search query string
    /// * `options` - Search configuration options
    ///
    /// # Returns
    /// A vector of search results where the query matched in the content
    pub fn search_content_with_options(
        &self,
        query: &str,
        options: &SearchOptions,
    ) -> Result<Vec<SearchResult>, OraError> {
        // For content-only search, we use FTS5 with column-specific syntax
        let content_query = format!("content:{}", query);
        self.search_with_options(&content_query, options)
    }

    /// Performs an advanced search using raw FTS5 query syntax.
    ///
    /// This method allows full control over the FTS5 query syntax for complex
    /// searches that might include boolean operators, phrase matching, and
    /// column-specific searches.
    ///
    /// # Arguments
    /// * `query` - Raw FTS5 query string
    /// * `options` - Search configuration options
    ///
    /// # Returns
    /// A vector of search results ranked by relevance
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use ora_core::search::{Query, SearchOptions};
    /// # use ora_core::watcher::index::Index;
    /// # use std::path::Path;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let index = Index::new(Path::new("/path/to/shelf"))?;
    /// # let query = Query::new(&index);
    /// // Complex boolean query
    /// let results = query.advanced_search(
    ///     "title:rust AND (programming OR tutorial) NOT beginner",
    ///     &SearchOptions::default()
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn advanced_search(
        &self,
        query: &str,
        options: &SearchOptions,
    ) -> Result<Vec<SearchResult>, OraError> {
        self.search_with_options(query, options)
    }

    /// Counts the total number of results for a query.
    ///
    /// Useful for implementing pagination UIs where you need to know the
    /// total number of matches before fetching a specific page.
    ///
    /// # Arguments
    /// * `query` - The search query string
    ///
    /// # Returns
    /// The total number of matching notes
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use ora_core::search::Query;
    /// # use ora_core::watcher::index::Index;
    /// # use std::path::Path;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let index = Index::new(Path::new("/path/to/shelf"))?;
    /// # let query = Query::new(&index);
    /// let total = query.count_results("rust")?;
    /// println!("Found {} notes matching 'rust'", total);
    /// # Ok(())
    /// # }
    /// ```
    pub fn count_results(&self, query: &str) -> Result<u64, OraError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT COUNT(*) as count
            FROM contents
            WHERE contents MATCH ?
            "#,
        )?;

        let count: i64 = stmt.query_row(params![query], |row| row.get(0))?;
        Ok(count as u64)
    }

    /// Provides auto-complete suggestions for note titles.
    ///
    /// Searches for note titles that start with the given prefix, useful
    /// for implementing auto-complete functionality in user interfaces.
    ///
    /// # Arguments
    /// * `prefix` - The prefix to match against note titles
    /// * `limit` - Maximum number of suggestions to return (defaults to 10)
    ///
    /// # Returns
    /// A vector of note titles that start with the prefix, sorted alphabetically
    ///
    /// # Examples
    /// ```rust,no_run
    /// # use ora_core::search::Query;
    /// # use ora_core::watcher::index::Index;
    /// # use std::path::Path;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # let index = Index::new(Path::new("/path/to/shelf"))?;
    /// # let query = Query::new(&index);
    /// let suggestions = query.suggest("rust", Some(5))?;
    /// // Might return: ["rust basics", "rust programming", "rust tutorial"]
    /// # Ok(())
    /// # }
    /// ```
    pub fn suggest(&self, prefix: &str, limit: Option<u32>) -> Result<Vec<String>, OraError> {
        let conn = self.conn.lock().unwrap();
        let limit = limit.unwrap_or(10);

        let mut stmt = conn.prepare(
            r#"
            SELECT DISTINCT title
            FROM notes
            WHERE title LIKE ? || '%'
            ORDER BY title
            LIMIT ?
            "#,
        )?;

        let rows = stmt.query_map(params![prefix, limit], |row| Ok(row.get::<_, String>(0)?))?;

        let mut suggestions = Vec::new();
        for row in rows {
            suggestions.push(row?);
        }

        Ok(suggestions)
    }
}
