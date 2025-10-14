use ora_core::error::OraError;
use ora_core::shelf::manager::ShelfManager;
use ora_core::shelf::storage::Shelf;
use tempfile::TempDir;

#[test]
fn create_and_list_notes() -> Result<(), OraError> {
    let tmpdir = TempDir::new()?;
    let shelf = Shelf {
        root: tmpdir.path().to_path_buf(),
        name: "test_shelf".to_string(),
    };

    let manager = ShelfManager::new(&shelf);

    // create two notes
    manager.create_note("First", "hello")?;
    manager.create_note("Second", "world")?;

    // list them back
    let listed = manager.list_notes()?;
    assert_eq!(listed.len(), 2);

    let titles: Vec<_> = listed.iter().map(|n| n.title.as_str()).collect();
    assert!(titles.contains(&"First"));
    assert!(titles.contains(&"Second"));

    Ok(())
}

#[test]
fn get_note_should_return_specific_note() -> Result<(), OraError> {
    let tmpdir = TempDir::new()?;
    let shelf = Shelf {
        root: tmpdir.path().to_path_buf(),
        name: "test_shelf".to_string(),
    };

    let manager = ShelfManager::new(&shelf);
    let note = manager.create_note("Special", "important data")?;

    // title comes from filename
    let title = note.path.file_stem().unwrap().to_string_lossy().to_string();

    let fetched = manager.get_note(&title)?;
    assert_eq!(fetched.title, "Special");
    assert!(fetched.content.contains("important data"));

    Ok(())
}

#[test]
fn update_note_should_change_content_and_title() -> Result<(), OraError> {
    let tmpdir = TempDir::new()?;
    let shelf = Shelf {
        root: tmpdir.path().to_path_buf(),
        name: "test_shelf".to_string(),
    };

    let manager = ShelfManager::new(&shelf);
    let note = manager.create_note("UpdateMe", "old text")?;

    let title = note.path.file_stem().unwrap().to_string_lossy().to_string();

    // update both content and title
    let updated = manager.update_note(&title, Some("UpdatedTitle"), Some("new text"))?;
    assert_eq!(updated.title, "UpdatedTitle");
    assert!(updated.content.contains("new text"));

    Ok(())
}

#[test]
fn delete_note_should_remove_file() -> Result<(), OraError> {
    let tmpdir = TempDir::new()?;
    let shelf = Shelf {
        root: tmpdir.path().to_path_buf(),
        name: "test_shelf".to_string(),
    };

    let manager = ShelfManager::new(&shelf);
    let note = manager.create_note("DeleteMe", "bye!")?;

    let title = note.path.file_stem().unwrap().to_string_lossy().to_string();

    manager.delete_note(&title)?;

    assert!(!note.path.exists());

    Ok(())
}
