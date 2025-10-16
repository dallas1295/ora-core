use crate::error::OraError;
use crate::watcher::index::{Index, IndexedNote};
use rusqlite::{Connection, params};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct Query {
    conn: Arc<Mutex<Connection>>,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub note: IndexedNote,
    pub rank: f64,
    pub snippet_title: Option<String>,
    pub snippet_content: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SearchOptions {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub include_snippets: bool,
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
    pub fn new(index: &Index) -> Self {
        Self {
            conn: index.conn.clone(),
        }
    }

    pub fn search(&self, query: &str) -> Result<Vec<SearchResult>, OraError> {
        self.search_with_options(query, &SearchOptions::default())
    }

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
        let rows = stmt.query_map(
            params![query, limit, offset],
            |row| {
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
            },
        )?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }

        Ok(results)
    }

    pub fn search_title(&self, query: &str) -> Result<Vec<SearchResult>, OraError> {
        self.search_title_with_options(query, &SearchOptions::default())
    }

    pub fn search_title_with_options(
        &self,
        query: &str,
        options: &SearchOptions,
    ) -> Result<Vec<SearchResult>, OraError> {
        // For title-only search, we use FTS5 with column-specific syntax
        let title_query = format!("title:{}", query);
        self.search_with_options(&title_query, options)
    }

    pub fn search_content(&self, query: &str) -> Result<Vec<SearchResult>, OraError> {
        self.search_content_with_options(query, &SearchOptions::default())
    }

    pub fn search_content_with_options(
        &self,
        query: &str,
        options: &SearchOptions,
    ) -> Result<Vec<SearchResult>, OraError> {
        // For content-only search, we use FTS5 with column-specific syntax
        let content_query = format!("content:{}", query);
        self.search_with_options(&content_query, options)
    }

    pub fn advanced_search(
        &self,
        query: &str,
        options: &SearchOptions,
    ) -> Result<Vec<SearchResult>, OraError> {
        self.search_with_options(query, options)
    }

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

        let rows = stmt.query_map(params![prefix, limit], |row| {
            Ok(row.get::<_, String>(0)?)
        })?;

        let mut suggestions = Vec::new();
        for row in rows {
            suggestions.push(row?);
        }

        Ok(suggestions)
    }
}
