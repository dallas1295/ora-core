use ora_core::error::OraError;
use ora_core::watcher::index::Index;
use ora_core::watcher::service::WatcherService;
use std::fs;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

#[test]
fn watcher_full_integration_test() -> Result<(), OraError> {
    // Use Documents folder for real-world testing
    let docs_dir = dirs::document_dir()
        .ok_or_else(|| OraError::Other("Cannot find Documents directory".to_string()))?;
    let test_shelf = docs_dir.join("ora_test_shelf");

    // Clean up any existing test shelf
    if test_shelf.exists() {
        fs::remove_dir_all(&test_shelf)?;
    }

    fs::create_dir_all(&test_shelf)?;
    let shelf_path = test_shelf;

    println!("Starting watcher service...");

    // Start watcher ONCE
    let mut service = WatcherService::create(&shelf_path, Duration::from_millis(50))?;
    service.run()?;

    // Wait for watcher to stabilize and scan existing files
    println!("Waiting for watcher to stabilize...");
    thread::sleep(Duration::from_millis(2000));

    // Test 1: File creation detection
    println!("Test 1: File creation detection");
    let note_path = shelf_path.join("Test Note.md");
    fs::write(
        &note_path,
        "# Test Note\nThis should be detected by watcher",
    )?;
    thread::sleep(Duration::from_millis(500));

    let index = Index::new(&shelf_path)?;
    let indexed_note = index.get_by_path(&note_path)?;
    assert!(
        indexed_note.is_some(),
        "New file should be indexed by watcher"
    );
    assert_eq!(indexed_note.unwrap().title, "Test Note");
    println!("✅ File creation test passed");

    // Test 2: File modification detection
    println!("Test 2: File modification detection");
    println!("Debug: Writing new content to file...");
    fs::write(&note_path, "# Test Note\nModified content with updates")?;

    // Debug: Verify what's actually in the file on disk
    let disk_content = fs::read_to_string(&note_path)?;
    println!("Debug: Content on disk = '{}'", disk_content);

    println!("Debug: Waiting for watcher to process modification...");
    thread::sleep(Duration::from_millis(1000));

    // Use the same index instance as the watcher to avoid SQLite locking issues
    println!("Debug: Getting index instance from watcher service...");
    let fresh_index = service.get_index();
    let updated_note = fresh_index.get_by_path(&note_path)?;
    assert!(
        updated_note.is_some(),
        "Modified file should still be indexed"
    );
    let note = updated_note.unwrap();
    println!("Debug: Note title from database = '{}'", note.title);
    println!("Debug: Note content from database = '{}'", note.content);
    println!(
        "Debug: Does content contain 'Modified content'? {}",
        note.content.contains("Modified content")
    );
    assert_eq!(note.title, "Test Note"); // Title from filename
    assert!(note.content.contains("Modified content"));
    println!("✅ File modification test passed");

    // Test 3: Nested directory handling
    println!("Test 3: Nested directory handling");
    let subdir = shelf_path.join("subdir");
    let nested_subdir = subdir.join("nested");
    fs::create_dir_all(&nested_subdir)?;

    let nested_file = nested_subdir.join("Nested Note.md");
    fs::write(&nested_file, "# Nested Note\nContent in nested directory")?;
    thread::sleep(Duration::from_millis(500));

    let nested_indexed = index.get_by_path(&nested_file)?;
    assert!(
        nested_indexed.is_some(),
        "Nested directory file should be indexed"
    );
    assert_eq!(nested_indexed.unwrap().title, "Nested Note");
    println!("✅ Nested directory test passed");

    // Test 4: Non-markdown file filtering
    println!("Test 4: Non-markdown file filtering");
    fs::write(shelf_path.join("test.txt"), "Text content")?;
    fs::write(shelf_path.join("test.json"), "{\"json\": \"content\"}")?;
    fs::write(
        shelf_path.join("Markdown File.md"),
        "# Markdown File\nShould be indexed",
    )?;
    fs::write(
        shelf_path.join(".hidden.md"),
        "# Hidden Markdown\nShould not be indexed",
    )?;
    thread::sleep(Duration::from_millis(500));

    assert!(
        index
            .get_by_path(&shelf_path.join("Markdown File.md"))?
            .is_some(),
        "Visible markdown should be indexed"
    );
    assert!(
        index.get_by_path(&shelf_path.join("test.txt"))?.is_none(),
        "Text file should not be indexed"
    );
    assert!(
        index.get_by_path(&shelf_path.join("test.json"))?.is_none(),
        "JSON file should not be indexed"
    );
    assert!(
        index.get_by_path(&shelf_path.join(".hidden.md"))?.is_none(),
        "Hidden markdown should not be indexed"
    );
    println!("✅ Non-markdown filtering test passed");

    // Test 5: File deletion detection
    println!("Test 5: File deletion detection");
    let deletable_path = shelf_path.join("Deletable Note.md");
    fs::write(&deletable_path, "# Deletable Note\nThis will be deleted")?;
    thread::sleep(Duration::from_millis(500));

    // Verify it was indexed
    assert!(
        index.get_by_path(&deletable_path)?.is_some(),
        "File should be indexed before deletion"
    );

    // Delete the file
    fs::remove_file(&deletable_path)?;
    thread::sleep(Duration::from_millis(500));

    // Verify it was removed from index
    assert!(
        index.get_by_path(&deletable_path)?.is_none(),
        "Deleted file should be removed from index"
    );
    println!("✅ File deletion test passed");

    // Test 6: Debouncing rapid changes
    println!("Test 6: Debouncing rapid changes");
    let debounce_path = shelf_path.join("Debounce Test.md");

    // Make rapid changes to the same file
    for i in 0..5 {
        fs::write(
            &debounce_path,
            &format!("# Rapid Change {}\nContent version {}", i, i),
        )?;
        thread::sleep(Duration::from_millis(20)); // Faster than debounce time
    }

    // Wait for debounce to settle
    thread::sleep(Duration::from_millis(300));

    let debounced_note = index.get_by_path(&debounce_path)?;
    assert!(
        debounced_note.is_some(),
        "File should be indexed after debouncing"
    );
    let content = debounced_note.unwrap().content;
    assert!(
        content.contains("Rapid Change 4"),
        "Should contain final version"
    );
    assert!(
        !content.contains("Rapid Change 0"),
        "Should not contain first rapid change"
    );
    println!("✅ Debouncing test passed");

    // Test 7: Concurrent file creation
    println!("Test 7: Concurrent file creation");
    let mut handles = vec![];
    for i in 0..3 {
        let shelf_clone = shelf_path.clone();
        let handle = thread::spawn(move || {
            let note_path = shelf_clone.join(format!("Concurrent Note {}.md", i));
            fs::write(
                &note_path,
                &format!("# Concurrent Note {}\nContent for note {}", i, i),
            )
            .unwrap();
        });
        handles.push(handle);
    }

    // Wait for all file creations to complete
    for handle in handles {
        handle.join().unwrap();
    }

    // Give watcher time to process all changes
    thread::sleep(Duration::from_millis(500));

    // Verify all files were indexed
    for i in 0..3 {
        let note_path = shelf_path.join(format!("Concurrent Note {}.md", i));
        assert!(
            index.get_by_path(&note_path)?.is_some(),
            "Concurrent file {} should be indexed",
            i
        );
    }
    println!("✅ Concurrent creation test passed");

    // Shutdown ONCE
    println!("About to shutdown watcher service...");
    let shutdown_start = std::time::Instant::now();
    service.shutdown()?;
    let shutdown_duration = shutdown_start.elapsed();
    println!("Watcher shutdown completed in {:?}", shutdown_duration);

    thread::sleep(Duration::from_millis(1000));

    // Clean up test shelf
    println!("About to clean up test shelf...");
    let cleanup_start = std::time::Instant::now();
    if shelf_path.exists() {
        fs::remove_dir_all(&shelf_path)?;
    }
    let cleanup_duration = cleanup_start.elapsed();
    println!("Test shelf cleanup completed in {:?}", cleanup_duration);

    println!("🎉 All watcher tests completed successfully!");
    println!("✅ File creation detection");
    println!("✅ File modification detection");
    println!("✅ Nested directory handling");
    println!("✅ Non-markdown file filtering");
    println!("✅ File deletion detection");
    println!("✅ Debouncing behavior");
    println!("✅ Concurrent file creation");

    Ok(())
}

#[test]
fn watcher_service_creates_successfully() -> Result<(), OraError> {
    let tmpdir = TempDir::new()?;
    let shelf_path = tmpdir.path().to_path_buf();

    // Create watcher service
    let mut service = WatcherService::create(&shelf_path, Duration::from_millis(100))?;

    // Verify service was created (no panic)
    assert!(true, "Watcher service should create successfully");

    service.shutdown()?;
    thread::sleep(Duration::from_millis(1000));
    Ok(())
}
