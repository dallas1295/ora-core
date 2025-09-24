use rough_core::domain::{LocalNote, NoteError};
use tempfile::TempDir;

#[test]
fn create_and_reload_note() -> Result<(), NoteError> {
    let tmpdir = TempDir::new().unwrap();
    let dir = tmpdir.path();

    // 1. Create a note, ensure file exists
    let note = LocalNote::create("Test Note".into(), "Hello, world".into(), dir)?;
    assert!(note.path.exists());

    // 2. Reload from disk and ensure content matches
    let reloaded = note.reload()?;
    assert_eq!(reloaded.content, "Hello, world");

    Ok(())
}

#[test]
fn update_content_and_save() -> Result<(), NoteError> {
    let tmpdir = TempDir::new().unwrap();
    let dir = tmpdir.path();

    let note = LocalNote::create("Content Note".into(), "Original".into(), dir)?;

    // 1. Make a new note in memory with updated content
    let updated = note.with_content("Updated".into());

    // 2. Persist the updated note
    updated.save()?;

    // 3. Reload from disk, check content
    let reloaded = updated.reload()?;
    assert_eq!(reloaded.content, "Updated");

    Ok(())
}

#[test]
fn update_title_and_persist_rename() -> Result<(), NoteError> {
    let tmpdir = TempDir::new().unwrap();
    let dir = tmpdir.path();

    let note = LocalNote::create("Title Note".into(), "data".into(), dir)?;

    // 1. Generate new note with new title
    let new_note = note.with_title("Renamed".into())?;

    // 2. Persist rename to disk
    new_note.persist_rename(&note.path)?;

    // old file should be gone, new should exist
    assert!(!note.path.exists());
    assert!(new_note.path.exists());

    Ok(())
}

#[test]
fn delete_removes_file() -> Result<(), NoteError> {
    let tmpdir = TempDir::new().unwrap();
    let dir = tmpdir.path();

    let note = LocalNote::create("Delete Note".into(), "to be deleted".into(), dir)?;
    assert!(note.path.exists());

    note.delete()?;
    assert!(!note.path.exists());

    Ok(())
}

#[test]
fn invalid_title_should_fail() {
    let tmpdir = TempDir::new().unwrap();
    let dir = tmpdir.path();

    let bad = LocalNote::create("bad/title".into(), "oops".into(), dir);
    assert!(matches!(bad, Err(NoteError::InvalidTitle)));

    let empty = LocalNote::create("    ".into(), "empty".into(), dir);
    assert!(matches!(empty, Err(NoteError::InvalidTitle)));
}
