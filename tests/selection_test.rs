//! Tests for text selection and clipboard functionality

use ai_c::{
    ui::selection::{SelectionManager, TextPosition, TextSelection, SelectionMode},
};

#[tokio::test]
async fn test_text_selection() {
    let mut manager = SelectionManager::new();

    // Test starting selection
    let pos1 = TextPosition::new(0, 5);
    manager.start_selection(pos1);

    assert!(manager.is_selecting());
    assert!(manager.get_selection().is_some());

    // Test updating selection
    let pos2 = TextPosition::new(0, 10);
    manager.update_selection(pos2);

    let selection = manager.get_selection().unwrap();
    assert_eq!(selection.start, pos1);
    assert_eq!(selection.end, pos2);

    // Test ending selection
    manager.end_selection();
    assert!(!manager.is_selecting());

    // Selection should still exist
    assert!(manager.get_selection().is_some());
}

#[tokio::test]
async fn test_text_extraction() {
    let manager = SelectionManager::new();
    let text_lines = vec![
        "Hello world".to_string(),
        "This is line 2".to_string(),
        "Final line".to_string(),
    ];

    // Test single line extraction
    let selection = TextSelection::new(
        TextPosition::new(0, 6),
        TextPosition::new(0, 11),
    );

    let extracted = manager.extract_selected_text(&text_lines, &selection).unwrap();
    assert_eq!(extracted, "world");

    // Test multi-line extraction
    let selection = TextSelection::new(
        TextPosition::new(0, 6),
        TextPosition::new(2, 5),
    );

    let extracted = manager.extract_selected_text(&text_lines, &selection).unwrap();
    assert_eq!(extracted, "world\nThis is line 2\nFinal");
}

#[tokio::test]
async fn test_select_all() {
    let mut manager = SelectionManager::new();
    let text_lines = vec![
        "Line 1".to_string(),
        "Line 2".to_string(),
        "Line 3".to_string(),
    ];

    manager.select_all(&text_lines);

    let selection = manager.get_selection().unwrap();
    assert_eq!(selection.start, TextPosition::new(0, 0));
    assert_eq!(selection.end, TextPosition::new(2, 6)); // "Line 3" has 6 characters
}

#[tokio::test]
async fn test_select_line() {
    let mut manager = SelectionManager::new();
    let text_lines = vec![
        "First line".to_string(),
        "Second line".to_string(),
        "Third line".to_string(),
    ];

    manager.select_line(1, &text_lines);

    let selection = manager.get_selection().unwrap();
    assert_eq!(selection.start, TextPosition::new(1, 0));
    assert_eq!(selection.end, TextPosition::new(1, 11)); // "Second line" has 11 characters
}

#[tokio::test]
async fn test_select_word() {
    let mut manager = SelectionManager::new();
    let text_lines = vec![
        "Hello world test".to_string(),
    ];

    // Select word "world" at position (0, 7)
    manager.select_word_at(TextPosition::new(0, 7), &text_lines);

    let selection = manager.get_selection().unwrap();
    assert_eq!(selection.start, TextPosition::new(0, 6)); // Start of "world"
    assert_eq!(selection.end, TextPosition::new(0, 11));  // End of "world"
}

#[test]
fn test_text_position_comparison() {
    let pos1 = TextPosition::new(1, 5);
    let pos2 = TextPosition::new(1, 10);
    let pos3 = TextPosition::new(2, 3);

    assert!(pos1.is_before(&pos2));
    assert!(pos1.is_before(&pos3));
    assert!(pos2.is_before(&pos3));

    assert!(pos2.is_after(&pos1));
    assert!(pos3.is_after(&pos1));
    assert!(pos3.is_after(&pos2));

    assert!(!pos1.is_before(&pos1)); // Same position
    assert!(!pos1.is_after(&pos1));  // Same position
}

#[test]
fn test_selection_contains() {
    let selection = TextSelection::new(
        TextPosition::new(1, 5),
        TextPosition::new(3, 10),
    );

    // Test positions within selection
    assert!(selection.contains(&TextPosition::new(1, 5)));  // Start
    assert!(selection.contains(&TextPosition::new(3, 10))); // End
    assert!(selection.contains(&TextPosition::new(2, 0)));  // Middle
    assert!(selection.contains(&TextPosition::new(1, 8)));  // First line
    assert!(selection.contains(&TextPosition::new(3, 5)));  // Last line

    // Test positions outside selection
    assert!(!selection.contains(&TextPosition::new(1, 4)));  // Before start
    assert!(!selection.contains(&TextPosition::new(3, 11))); // After end
    assert!(!selection.contains(&TextPosition::new(0, 10))); // Before start line
    assert!(!selection.contains(&TextPosition::new(4, 0)));  // After end line
}

#[tokio::test]
async fn test_clipboard_availability() {
    let manager = SelectionManager::new();

    // Just test that clipboard can be checked for availability
    // This might fail in CI environments without a display, which is expected
    let _is_available = manager.clipboard.is_available().await;

    // The test passes if we don't panic
}

#[test]
fn test_selection_mode() {
    let mut manager = SelectionManager::new();

    assert_eq!(manager.get_mode(), SelectionMode::Character);

    manager.set_mode(SelectionMode::Line);
    assert_eq!(manager.get_mode(), SelectionMode::Line);

    manager.set_mode(SelectionMode::Word);
    assert_eq!(manager.get_mode(), SelectionMode::Word);

    manager.set_mode(SelectionMode::Block);
    assert_eq!(manager.get_mode(), SelectionMode::Block);
}