use ora_core::domain::LocalNote;
use ora_core::search::index::Index;
use ora_core::search::query::{Query, SearchOptions};
use tempfile::TempDir;

#[tokio::test]
async fn basic_search_returns_matching_notes() -> Result<(), ora_core::error::OraError> {
    let tmpdir = TempDir::new().unwrap();
    let shelf_path = tmpdir.path();

    let index = Index::new(shelf_path).await?;
    let query = Query::new(&index);

    // Create test notes
    let note1 = LocalNote::create(
        "Rust Programming",
        "Learn about Rust language features",
        shelf_path,
    )
    .map_err(|e| ora_core::error::OraError::Other(e.to_string()))?;
    let note2 = LocalNote::create("Python Guide", "Python programming tutorial", shelf_path)
        .map_err(|e| ora_core::error::OraError::Other(e.to_string()))?;
    let note3 = LocalNote::create(
        "JavaScript Tips",
        "Advanced JavaScript techniques",
        shelf_path,
    )
    .map_err(|e| ora_core::error::OraError::Other(e.to_string()))?;

    // Index the notes
    index.index_note(&note1).await?;
    index.index_note(&note2).await?;
    index.index_note(&note3).await?;

    // Search for "Rust"
    let results = query.search("Rust").await?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].note.title, "Rust Programming");
    assert!(results[0].note.content.contains("Rust"));

    // Search for "programming"
    let results = query.search("programming").await?;
    assert_eq!(results.len(), 2); // Should match Rust and Python notes

    Ok(())
}

#[tokio::test]
async fn title_only_search() -> Result<(), ora_core::error::OraError> {
    let tmpdir = TempDir::new().unwrap();
    let shelf_path = tmpdir.path();

    let index = Index::new(shelf_path).await?;
    let query = Query::new(&index);

    // Create notes with "Guide" in content but not title
    let note1 = LocalNote::create(
        "Rust Book",
        "This is a comprehensive guide to Rust",
        shelf_path,
    )
    .map_err(|e| ora_core::error::OraError::Other(e.to_string()))?;
    let note2 = LocalNote::create("Python Guide", "Learn Python programming", shelf_path)
        .map_err(|e| ora_core::error::OraError::Other(e.to_string()))?;

    index.index_note(&note1).await?;
    index.index_note(&note2).await?;

    // Search titles for "Guide"
    let results = query.search_title("Guide").await?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].note.title, "Python Guide");

    Ok(())
}

#[tokio::test]
async fn content_only_search() -> Result<(), ora_core::error::OraError> {
    let tmpdir = TempDir::new().unwrap();
    let shelf_path = tmpdir.path();

    let index = Index::new(shelf_path).await?;
    let query = Query::new(&index);

    // Create notes
    let note1 = LocalNote::create("Rust", "This is about comprehensive learning", shelf_path)
        .map_err(|e| ora_core::error::OraError::Other(e.to_string()))?;
    let note2 = LocalNote::create("Python", "Python programming tutorial", shelf_path)
        .map_err(|e| ora_core::error::OraError::Other(e.to_string()))?;

    index.index_note(&note1).await?;
    index.index_note(&note2).await?;

    // Search content for "comprehensive"
    let results = query.search_content("comprehensive").await?;
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].note.title, "Rust");

    Ok(())
}

#[tokio::test]
async fn search_with_pagination() -> Result<(), ora_core::error::OraError> {
    let tmpdir = TempDir::new().unwrap();
    let shelf_path = tmpdir.path();

    let index = Index::new(shelf_path).await?;
    let query = Query::new(&index);

    // Create multiple notes
    for i in 1..=10 {
        let note = LocalNote::create(
            &format!("Note {}", i),
            &format!("Content for note number {}", i),
            shelf_path,
        )
        .map_err(|e| ora_core::error::OraError::Other(e.to_string()))?;
        index.index_note(&note).await?;
    }

    // Get first page
    let options = SearchOptions {
        limit: Some(3),
        offset: Some(0),
        include_snippets: false,
        snippet_length: 100,
    };
    let page1 = query.search_with_options("Note", &options).await?;
    assert_eq!(page1.len(), 3);

    // Get second page
    let options = SearchOptions {
        limit: Some(3),
        offset: Some(3),
        include_snippets: false,
        snippet_length: 100,
    };
    let page2 = query.search_with_options("Note", &options).await?;
    assert_eq!(page2.len(), 3);

    // Ensure different results
    let page1_titles: Vec<_> = page1.iter().map(|r| &r.note.title).collect();
    let page2_titles: Vec<_> = page2.iter().map(|r| &r.note.title).collect();
    assert!(!page1_titles.iter().any(|t| page2_titles.contains(t)));

    Ok(())
}

#[tokio::test]
async fn search_with_snippets() -> Result<(), ora_core::error::OraError> {
    let tmpdir = TempDir::new().unwrap();
    let shelf_path = tmpdir.path();

    let index = Index::new(shelf_path).await?;
    let query = Query::new(&index);

    let note = LocalNote::create(
        "Rust Programming",
        "Rust is a systems programming language that runs blazingly fast",
        shelf_path,
    )
    .map_err(|e| ora_core::error::OraError::Other(e.to_string()))?;

    index.index_note(&note).await?;

    // Search with snippets
    let options = SearchOptions {
        limit: Some(10),
        offset: Some(0),
        include_snippets: true,
        snippet_length: 50,
    };
    let results = query.search_with_options("blazingly", &options).await?;
    assert_eq!(results.len(), 1);

    let result = &results[0];
    assert!(result.snippet_content.is_some());
    assert!(
        result
            .snippet_content
            .as_ref()
            .unwrap()
            .contains("<mark>blazingly</mark>")
    );

    Ok(())
}

#[tokio::test]
async fn count_search_results() -> Result<(), ora_core::error::OraError> {
    let tmpdir = TempDir::new().unwrap();
    let shelf_path = tmpdir.path();

    let index = Index::new(shelf_path).await?;
    let query = Query::new(&index);

    // Create notes with "programming" in them
    let note1 = LocalNote::create("Rust Programming", "Learn Rust programming", shelf_path)
        .map_err(|e| ora_core::error::OraError::Other(e.to_string()))?;
    let note2 = LocalNote::create("Python Programming", "Python programming guide", shelf_path)
        .map_err(|e| ora_core::error::OraError::Other(e.to_string()))?;
    let note3 = LocalNote::create("JavaScript", "Web development", shelf_path)
        .map_err(|e| ora_core::error::OraError::Other(e.to_string()))?;

    index.index_note(&note1).await?;
    index.index_note(&note2).await?;
    index.index_note(&note3).await?;

    let count = query.count_results("programming").await?;
    assert_eq!(count, 2);

    let count = query.count_results("JavaScript").await?;
    assert_eq!(count, 1);

    let count = query.count_results("nonexistent").await?;
    assert_eq!(count, 0);

    Ok(())
}

#[tokio::test]
async fn suggest_titles() -> Result<(), ora_core::error::OraError> {
    let tmpdir = TempDir::new().unwrap();
    let shelf_path = tmpdir.path();

    let index = Index::new(shelf_path).await?;
    let query = Query::new(&index);

    // Create notes with similar titles
    let note1 = LocalNote::create("Rust Programming", "Learn Rust", shelf_path)
        .map_err(|e| ora_core::error::OraError::Other(e.to_string()))?;
    let note2 = LocalNote::create("Rust Guide", "Rust tutorial", shelf_path)
        .map_err(|e| ora_core::error::OraError::Other(e.to_string()))?;
    let note3 = LocalNote::create("Python Programming", "Learn Python", shelf_path)
        .map_err(|e| ora_core::error::OraError::Other(e.to_string()))?;

    index.index_note(&note1).await?;
    index.index_note(&note2).await?;
    index.index_note(&note3).await?;

    let suggestions = query.suggest("Rust", Some(10)).await?;
    assert_eq!(suggestions.len(), 2);
    assert!(suggestions.contains(&"Rust Programming".to_string()));
    assert!(suggestions.contains(&"Rust Guide".to_string()));

    let suggestions = query.suggest("Py", Some(10)).await?;
    assert_eq!(suggestions.len(), 1);
    assert_eq!(suggestions[0], "Python Programming");

    Ok(())
}

#[tokio::test]
async fn advanced_fts5_queries() -> Result<(), ora_core::error::OraError> {
    let tmpdir = TempDir::new().unwrap();
    let shelf_path = tmpdir.path();

    let index = Index::new(shelf_path).await?;
    let query = Query::new(&index);

    // Create test notes
    let note1 = LocalNote::create(
        "Rust Programming",
        "Learn about Rust and memory safety",
        shelf_path,
    )
    .map_err(|e| ora_core::error::OraError::Other(e.to_string()))?;
    let note2 = LocalNote::create(
        "Python Guide",
        "Python programming and data science",
        shelf_path,
    )
    .map_err(|e| ora_core::error::OraError::Other(e.to_string()))?;
    let note3 = LocalNote::create(
        "Systems Programming",
        "Low-level programming in C and Rust",
        shelf_path,
    )
    .map_err(|e| ora_core::error::OraError::Other(e.to_string()))?;

    index.index_note(&note1).await?;
    index.index_note(&note2).await?;
    index.index_note(&note3).await?;

    // Boolean AND query
    let results = query.search("Rust AND programming").await?;
    assert!(results.len() >= 1);
    // Should find "Rust Programming" and possibly "Systems Programming" (contains both terms)

    // OR query
    let results = query.search("Rust OR Python").await?;
    assert!(results.len() >= 2); // Should find at least Rust and Python notes

    // NOT query
    let results = query.search("programming NOT Python").await?;
    assert!(results.len() >= 1); // Should find at least Rust Programming, possibly others

    Ok(())
}

#[tokio::test]
async fn empty_search_returns_no_results() -> Result<(), ora_core::error::OraError> {
    let tmpdir = TempDir::new().unwrap();
    let shelf_path = tmpdir.path();

    let index = Index::new(shelf_path).await?;
    let query = Query::new(&index);

    // Create a note
    let note = LocalNote::create("Test Note", "Some content", shelf_path)
        .map_err(|e| ora_core::error::OraError::Other(e.to_string()))?;
    index.index_note(&note).await?;

    // Search for non-existent term
    let results = query.search("nonexistentterm12345").await?;
    assert_eq!(results.len(), 0);

    let count = query.count_results("nonexistentterm12345").await?;
    assert_eq!(count, 0);

    Ok(())
}
