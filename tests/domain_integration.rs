use ora_core::domain::{LocalNote, NoteError};
use tempfile::TempDir;

#[test]
fn create_and_reload_note() -> Result<(), NoteError> {
    let tmpdir = TempDir::new().unwrap();
    let dir = tmpdir.path();

    let note = LocalNote::create("Test Note".into(), "Hello, world".into(), dir)?;
    assert!(note.path.exists());

    assert_eq!(note.content, "Hello, world");

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
fn save_as_renames_note() -> Result<(), NoteError> {
    let tmpdir = TempDir::new().unwrap();
    let dir = tmpdir.path();

    let mut note = LocalNote::create("Title Note".into(), "data".into(), dir)?;
    let old_path = note.path.clone();

    note.save_as("Renamed")?;

    assert!(!old_path.exists());
    assert!(note.path.exists());
    assert_eq!(note.content, "data");
    assert_eq!(note.title, "Renamed");
    assert!(note.path.ends_with("Renamed.md"));

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
fn special_characters_in_title() -> Result<(), NoteError> {
    let tmpdir = TempDir::new().unwrap();
    let dir = tmpdir.path();

    let note = LocalNote::create("Note: Special Characters!".into(), "oops".into(), dir)?;
    assert_eq!(note.title, "Note: Special Characters!");
    assert!(note.path.ends_with("Note: Special Characters!.md"));

    Ok(())
}

#[test]
fn empty_title_defaults_to_untitled() -> Result<(), NoteError> {
    let tmpdir = TempDir::new().unwrap();
    let dir = tmpdir.path();

    let untitled = LocalNote::create("    ".into(), "empty".into(), dir)?;
    assert_eq!(untitled.title, "Untitled");
    assert!(untitled.path.ends_with("Untitled.md"));

    Ok(())
}

#[test]
fn duplicate_titles_get_number_suffix() -> Result<(), NoteError> {
    let tmpdir = TempDir::new().unwrap();
    let dir = tmpdir.path();

    let note1 = LocalNote::create("Same Title".into(), "first".into(), dir)?;
    assert_eq!(note1.title, "Same Title");
    assert!(note1.path.ends_with("Same Title.md"));

    let note2 = LocalNote::create("Same Title".into(), "second".into(), dir)?;
    assert_eq!(note2.title, "Same Title");
    assert!(note2.path.ends_with("Same Title 1.md"));

    let note3 = LocalNote::create("Same Title".into(), "third".into(), dir)?;
    assert_eq!(note3.title, "Same Title");
    assert!(note3.path.ends_with("Same Title 2.md"));

    Ok(())
}
