use super::{eval, lua_with_modules};

const SETUP: &str = r#"
    require("main")
    local floating = require("floating")
    local function floating_window(name)
        local window = map_window(name, "app")
        window.x, window.y, window.width, window.height = 100, 100, 400, 300
        floating.set_floating(window, true)
        return window
    end
"#;

fn run<T: mlua::FromLua>(body: &str) -> T {
    eval(&lua_with_modules(), &format!("{SETUP}{body}"))
}

#[test]
fn a_floating_window_gets_a_title_bar_with_its_title() {
    let result: Vec<String> = run(r#"
        local window = floating_window("terminal")
        local kinds = {}
        for _, shape in ipairs(window.shapes) do table.insert(kinds, shape.spec.kind .. ":" .. tostring(shape.spec.text)) end
        return kinds
    "#);
    assert_eq!(result, ["rect:nil", "rect:nil", "text:terminal", "text:×"]);
}

#[test]
fn tiled_windows_have_no_title_bar() {
    let count: i64 = run(r#"
        local window = map_window("tiled", "app")
        return #window.shapes
    "#);
    assert_eq!(count, 0);
}

#[test]
fn the_title_bar_goes_when_the_window_stops_floating_or_goes_fullscreen() {
    let result: Vec<i64> = run(r#"
        local window = floating_window("a")
        local with_bar = #window.shapes
        require("requests").set_fullscreen(window, true)
        local fullscreen = #window.shapes
        require("requests").set_fullscreen(window, false)
        local back = #window.shapes
        floating.set_floating(window, false)
        return { with_bar, fullscreen, back, #window.shapes }
    "#);
    assert_eq!(result, [4, 0, 4, 0]);
}

#[test]
fn a_new_title_changes_only_the_label() {
    let result: Vec<i64> = run(r#"
        local window = floating_window("old")
        window.name = "new"
        wm.windows:emit("title_changed", window)
        local updates = {}
        for _, shape in ipairs(window.shapes) do table.insert(updates, shape.updates) end
        return updates
    "#);
    assert_eq!(result, [0, 0, 1, 0]);
}

#[test]
fn a_resize_event_updates_the_bar_width() {
    let result: Vec<i64> = run(r#"
        local window = floating_window("a")
        window.width = 600
        wm.windows:emit("resized", window)
        return { window.shapes[1].spec.width, window.shapes[3].spec.max_width, window.shapes[4].spec.x }
    "#);
    assert_eq!(result[0], 604);
    assert!(result[1] < 600 && result[1] > 500);
    assert!(result[2] > 570);
}

#[test]
fn a_resize_to_the_same_width_does_not_touch_the_bar() {
    let updates: i64 = run(r#"
        local window = floating_window("a")
        wm.windows:emit("resized", window)
        wm.windows:emit("resized", window)
        local total = 0
        for _, shape in ipairs(window.shapes) do total = total + shape.updates end
        return total
    "#);
    assert_eq!(updates, 0);
}

#[test]
fn focus_changes_the_colours() {
    let result: Vec<String> = run(r#"
        local window = floating_window("a")
        local other = floating_window("b")
        seat0:focus(window)
        local focused = window.shapes[1].spec.fill
        seat0:focus(other)
        return { focused, window.shapes[1].spec.fill, other.shapes[1].spec.fill }
    "#);
    assert_eq!(result[0], "#88c0d0");
    assert_eq!(result[1], "#3b4252");
    assert_eq!(result[2], "#88c0d0");
}

#[test]
fn pressing_the_title_bar_starts_a_move_and_is_not_passed_to_clients() {
    let result: Vec<String> = run(r#"
        local window = floating_window("a")
        local pointer = new_pointer(150, 90)
        local press = press_button(pointer, "Pressed", Button.Left)
        move_pointer(pointer, 250, 90)
        return { tostring(press.prevented), string.format("%d", window.x), tostring(pointer.is_grabbed) }
    "#);
    assert_eq!(result, ["true", "200", "true"]);
}

#[test]
fn releasing_the_button_ends_the_move() {
    let result: Vec<String> = run(r#"
        local window = floating_window("a")
        local pointer = new_pointer(150, 90)
        press_button(pointer, "Pressed", Button.Left)
        press_button(pointer, "Released", Button.Left)
        move_pointer(pointer, 300, 90)
        return { string.format("%d", window.x), tostring(pointer.is_grabbed) }
    "#);
    assert_eq!(result, ["100", "false"]);
}

#[test]
fn pressing_the_close_button_asks_the_window_to_close() {
    let result: Vec<bool> = run(r#"
        local window = floating_window("a")
        local pointer = new_pointer(495, 90)
        press_button(pointer, "Pressed", Button.Left)
        return { window.close_requested == true, pointer.is_grabbed }
    "#);
    assert_eq!(result, [true, false]);
}

#[test]
fn a_press_beside_the_title_bar_is_left_alone() {
    let prevented: bool = run(r#"
        floating_window("a")
        local press = press_button(new_pointer(150, 40), "Pressed", Button.Left)
        return press.prevented == true
    "#);
    assert!(!prevented);
}

#[test]
fn a_window_in_front_covers_the_title_bar_below_it() {
    let prevented: bool = run(r#"
        floating_window("below")
        local above = floating_window("above")
        above.x, above.y = 100, 60
        above.width, above.height = 400, 300
        wm.windows:emit("focus", nil, seat0)
        under = above
        local press = press_button(new_pointer(150, 90), "Pressed", Button.Left)
        return press.prevented == true
    "#);
    assert!(!prevented);
}

#[test]
fn pressing_the_title_bar_does_not_raise_the_window_beneath_it() {
    let result: Vec<bool> = run(r#"
        local tiled = map_window("tiled", "app")
        local window = floating_window("a")
        under = tiled
        local pointer = new_pointer(150, 90)
        press_button(pointer, "Pressed", Button.Left)
        return { wm.windows:topmost() == window, wm.windows:topmost() == tiled }
    "#);
    assert_eq!(result, [true, false]);
}
