use super::{eval, lua_with_modules};

const SETUP: &str = r#"
    require("main")
    require("tiling").set_gaps(0, 0)
    local a, b = new_window("a"), new_window("b")
    local pointer = new_pointer(300, 300)
    local super = { logo = true }
"#;

fn run<T: mlua::FromLua>(body: &str) -> T {
    eval(&lua_with_modules(), &format!("{SETUP}{body}"))
}

#[test]
fn super_drag_moves_a_window_and_floats_it() {
    let result: Vec<i64> = run(r#"
        under = b
        local start_x, start_y = b.x, b.y
        press_button(pointer, "Pressed", Button.Left, super)
        local grabbed = pointer.is_grabbed
        move_pointer(pointer, 350, 340)
        press_button(pointer, "Released", Button.Left, super)
        return { b.x - start_x, b.y - start_y, grabbed and 1 or 0, pointer.is_grabbed and 1 or 0, a.width }
    "#);
    assert_eq!(result, [50, 40, 1, 0, 1000]);
}

#[test]
fn a_press_without_the_modifier_is_left_to_the_client() {
    let result: Vec<bool> = run(r#"
        under = b
        local event = press_button(pointer, "Pressed", Button.Left, {})
        return { event.prevented == true, pointer.is_grabbed }
    "#);
    assert_eq!(result, [false, false]);
}

#[test]
fn the_drag_ends_only_when_every_button_is_up() {
    let result: Vec<bool> = run(r#"
        under = b
        press_button(pointer, "Pressed", Button.Left, super)
        press_button(pointer, "Pressed", Button.Middle, {})
        press_button(pointer, "Released", Button.Left, super)
        local still = pointer.is_grabbed
        local release = press_button(pointer, "Released", Button.Middle, {})
        return { still, pointer.is_grabbed, release.prevented == true }
    "#);
    assert_eq!(result, [true, false, true]);
}

#[test]
fn super_right_drag_resizes_from_the_nearest_corner() {
    let result: Vec<i64> = run(r#"
        b:set_position(Point(200, 200))
        b:set_size({ width = 400, height = 300 })
        under = b
        pointer.x, pointer.y = 550, 450
        press_button(pointer, "Pressed", Button.Right, super)
        move_pointer(pointer, 600, 500)
        press_button(pointer, "Released", Button.Right, super)
        local after_corner = { b.x, b.y, b.width, b.height }

        pointer.x, pointer.y = 250, 250
        press_button(pointer, "Pressed", Button.Right, super)
        move_pointer(pointer, 300, 270)
        press_button(pointer, "Released", Button.Right, super)
        return { after_corner[1], after_corner[2], after_corner[3], after_corner[4], b.x, b.y, b.width, b.height }
    "#);
    assert_eq!(result, [200, 200, 450, 350, 250, 220, 400, 330]);
}

#[test]
fn resizing_stops_at_the_minimum_size() {
    let result: Vec<i64> = run(r#"
        b:set_position(Point(200, 200))
        b:set_size({ width = 400, height = 300 })
        under = b
        pointer.x, pointer.y = 550, 450
        press_button(pointer, "Pressed", Button.Right, super)
        move_pointer(pointer, -2000, -2000)
        press_button(pointer, "Released", Button.Right, super)
        return { b.width, b.height }
    "#);
    assert_eq!(result, [100, 60]);
}

#[test]
fn floating_windows_leave_the_layout_and_return_on_toggle() {
    let result: Vec<i64> = run(r#"
        local floating = require("floating")
        floating.set_floating(b, true)
        local alone = a.width
        floating.set_floating(b, false)
        return { alone, a.width }
    "#);
    assert_eq!(result, [1000, 600]);
}

#[test]
fn fullscreen_windows_cannot_be_dragged() {
    let result: Vec<bool> = run(r#"
        b:states():set("fullscreen", true)
        under = b
        local event = press_button(pointer, "Pressed", Button.Left, super)
        return { event.prevented == true, pointer.is_grabbed }
    "#);
    assert_eq!(result, [false, false]);
}

#[test]
fn client_resize_request_uses_the_requested_edges() {
    let result: Vec<i64> = run(r#"
        b:set_position(Point(200, 200))
        b:set_size({ width = 400, height = 300 })
        press_button(pointer, "Pressed", Button.Left, {})
        wm.windows:emit("resize_request", { window = b, pointer = pointer, edges = "top_left" })
        move_pointer(pointer, 350, 330)
        return { b.x, b.y, b.width, b.height, pointer.is_grabbed and 1 or 0 }
    "#);
    assert_eq!(result, [250, 230, 350, 270, 1]);
}

#[test]
fn closing_the_window_being_dragged_releases_the_pointer() {
    let grabbed: bool = run(r#"
        under = b
        press_button(pointer, "Pressed", Button.Left, super)
        close_window(b)
        return pointer.is_grabbed
    "#);
    assert!(!grabbed);
}

#[test]
fn dialogs_float_next_to_their_parent() {
    let result: Vec<i64> = run(r#"
        a:set_position(Point(100, 100))
        b.parent_window = a
        wm.windows:emit("parent_changed", b)
        return { b.x, b.y, b.width, a.width }
    "#);
    assert_eq!(result, [140, 140, 0, 1000]);
}
