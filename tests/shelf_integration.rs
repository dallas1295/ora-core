use rough_core::shelf::Shelf;
use std::fs;
use uuid::Uuid;

/// Helper: generate a unique name for test shelves so runs don't collide
fn test_shelf_name(name: &str) -> String {
    format!("test_{}_{}", name, Uuid::new_v4())
}

#[test]
fn creating_new_shelf_should_create_directory() {
    let name = test_shelf_name("create");
    let shelf = Shelf::new(&name).expect("Should be able to create shelf");

    assert!(shelf.root.exists());
    assert!(shelf.root.is_dir());

    // cleanup after test
    fs::remove_dir_all(shelf.root).unwrap();
}

#[test]
fn opening_nonexistent_shelf_should_fail() {
    let name = test_shelf_name("open_missing");
    let result = Shelf::open(&name);

    assert!(result.is_err());
}

#[test]
fn ensure_will_create_if_missing() {
    let name = test_shelf_name("ensure");
    let shelf = Shelf::ensure_exists(&name).expect("ensure should create shelf");

    assert!(shelf.root.exists());

    // cleanup
    fs::remove_dir_all(shelf.root).unwrap();
}
