use rough_core::domain::{LocalNote, NoteError};
use tempfile::TempDir;

#[test]
fn create_and_reload_note() -> Result<(), NoteError> {
    let tmpdir = TempDir::new().unwrap();
    let dir = tmpdir.path();

    let note = LocalNote::create("Test Note".into(), "Hello, world".into(), dir)?;
    assert!(note.path.exists());

    assert!(note.content.starts_with("# Test Note"));

    let reloaded = note.reload()?;
    assert!(reloaded.content.contains("Hello, world"));
    assert_eq!(reloaded.title, "Test Note");

    Ok(())
}

#[test]
fn update_content_and_save() -> Result<(), NoteError> {
    let tmpdir = TempDir::new().unwrap();
    let dir = tmpdir.path();

    let note = LocalNote::create("Content Note".into(), "Original".into(), dir)?;

    let updated = note.with_content("Updated".into());
    updated.save()?;

    let reloaded = updated.reload()?;
    assert_eq!(reloaded.content, "Updated");

    Ok(())
}

#[test]
fn update_title_and_persist_rename() -> Result<(), NoteError> {
    let tmpdir = TempDir::new().unwrap();
    let dir = tmpdir.path();

    let note = LocalNote::create("Title Note".into(), "data".into(), dir)?;

    let new_note = note.with_title("Renamed".into())?;
    new_note.persist_rename(&note.path)?;

    assert!(!note.path.exists());
    assert!(new_note.path.exists());
    assert!(new_note.content.starts_with("# Renamed"));
    assert_eq!(new_note.title, "Renamed");
    assert!(new_note.path.ends_with("renamed.md"));

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
fn invalid_title_is_sanitized() -> Result<(), NoteError> {
    let tmpdir = TempDir::new().unwrap();
    let dir = tmpdir.path();

    let bad = LocalNote::create("bad/title".into(), "oops".into(), dir)?;
    assert_eq!(bad.title, "bad_title");
    assert!(bad.path.ends_with("bad_title.md"));

    Ok(())
}

#[test]
fn empty_title_defaults_to_untitled() -> Result<(), NoteError> {
    let tmpdir = TempDir::new().unwrap();
    let dir = tmpdir.path();

    let untitled = LocalNote::create("    ".into(), "empty".into(), dir)?;
    assert_eq!(untitled.title, "Untitled");
    assert!(untitled.path.ends_with("untitled.md"));

    Ok(())
}
