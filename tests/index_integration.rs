use rough_core::domain::LocalNote;
use rough_core::search::index::Index;
use tempfile::TempDir;

#[tokio::test]
async fn create_index_and_index_note() -> Result<(), rough_core::error::RoughError> {
    let tmpdir = TempDir::new().unwrap();
    let shelf_path = tmpdir.path();

    // Create index
    let index = Index::new(shelf_path).await?;

    // Create a note
    let note = LocalNote::create("Test Note", "Hello world content", shelf_path)
        .map_err(|e| rough_core::error::RoughError::Other(e.to_string()))?;

    // Index the note
    index.index_note(&note).await?;

    // Retrieve by path
    let indexed = index.get_by_path(&note.path).await?;
    assert!(indexed.is_some());

    let indexed_note = indexed.unwrap();
    assert_eq!(indexed_note.title, "Test Note");
    assert!(indexed_note.content.contains("Hello world"));
    assert_eq!(indexed_note.path, note.path);

    Ok(())
}

#[tokio::test]
async fn get_nonexistent_note_returns_none() -> Result<(), rough_core::error::RoughError> {
    let tmpdir = TempDir::new().unwrap();
    let shelf_path = tmpdir.path();

    let index = Index::new(shelf_path).await?;

    // Try to get a note that doesn't exist
    let fake_path = shelf_path.join("nonexistent.md");
    let result = index.get_by_path(&fake_path).await?;

    assert!(result.is_none());

    Ok(())
}

#[tokio::test]
async fn remove_note_should_delete_from_index() -> Result<(), rough_core::error::RoughError> {
    let tmpdir = TempDir::new().unwrap();
    let shelf_path = tmpdir.path();

    let index = Index::new(shelf_path).await?;

    // Create and index a note
    let note = LocalNote::create("Delete Me", "Content to delete", shelf_path)
        .map_err(|e| rough_core::error::RoughError::Other(e.to_string()))?;

    index.index_note(&note).await?;

    // Verify it's indexed
    let indexed = index.get_by_path(&note.path).await?;
    assert!(indexed.is_some());

    // Remove from index
    let was_removed = index.remove_note(&note).await?;
    assert!(was_removed);

    // Verify it's gone
    let indexed = index.get_by_path(&note.path).await?;
    assert!(indexed.is_none());

    Ok(())
}

#[tokio::test]
async fn remove_nonexistent_note_returns_false() -> Result<(), rough_core::error::RoughError> {
    let tmpdir = TempDir::new().unwrap();
    let shelf_path = tmpdir.path();

    let index = Index::new(shelf_path).await?;

    // Create a note but don't index it
    let note = LocalNote::create("Not Indexed", "Content", shelf_path)
        .map_err(|e| rough_core::error::RoughError::Other(e.to_string()))?;

    // Try to remove from index
    let was_removed = index.remove_note(&note).await?;
    assert!(!was_removed);

    Ok(())
}

#[tokio::test]
async fn update_note_should_replace_in_index() -> Result<(), rough_core::error::RoughError> {
    let tmpdir = TempDir::new().unwrap();
    let shelf_path = tmpdir.path();

    let index = Index::new(shelf_path).await?;

    // Create and index a note
    let note = LocalNote::create("Original Title", "Original content", shelf_path)
        .map_err(|e| rough_core::error::RoughError::Other(e.to_string()))?;

    index.index_note(&note).await?;

    // Verify original content
    let indexed = index.get_by_path(&note.path).await?;
    assert!(indexed.is_some());
    assert_eq!(indexed.unwrap().title, "Original Title");

    // Update the note content (keeping same path)
    let updated_note = note.with_content("Updated content");

    index.index_note(&updated_note).await?;

    // Verify updated content (same path, different content)
    let indexed = index.get_by_path(&note.path).await?;
    assert!(indexed.is_some());
    let indexed_note = indexed.unwrap();
    assert_eq!(indexed_note.title, "Original Title"); // Title unchanged
    assert!(indexed_note.content.contains("Updated content"));

    Ok(())
}
