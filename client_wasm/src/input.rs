//! Keyboard input handling

use web_sys::KeyboardEvent;

/// Handle key down event
pub fn handle_key_down(key: &str, current_dir: i8) -> i8 {
    match key {
        "ArrowUp" | "w" | "W" => -1,
        "ArrowDown" | "s" | "S" => 1,
        _ => current_dir,
    }
}

/// Handle key up event
pub fn handle_key_up(key: &str, current_dir: i8) -> i8 {
    match key {
        "ArrowUp" | "w" | "W" | "ArrowDown" | "s" | "S" => 0,
        _ => current_dir,
    }
}

/// Extract key from keyboard event
pub fn get_key_from_event(event: &KeyboardEvent) -> String {
    event.key()
}
