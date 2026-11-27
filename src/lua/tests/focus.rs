use super::{eval, lua_with_modules};

#[test]
fn a_new_window_gets_focus() {
    let lua = lua_with_modules();
    let name: String = eval(
        &lua,
        r#"
        require("focus")
        new_window("a")
        new_window("b")
        return seat0:focused().name
        "#,
    );
    assert_eq!(name, "b");
}

#[test]
fn closing_the_focused_window_focuses_the_topmost_one() {
    let lua = lua_with_modules();
    let name: String = eval(
        &lua,
        r#"
        require("focus")
        local a, b, c = new_window("a"), new_window("b"), new_window("c")
        a:raise()
        seat0:focus(c)
        close_window(c)
        return seat0:focused().name
        "#,
    );
    assert_eq!(name, "a");
}

#[test]
fn closing_a_window_does_not_take_focus_from_a_layer() {
    let lua = lua_with_modules();
    let on_layer: bool = eval(
        &lua,
        r#"
        require("focus")
        local a, b = new_window("a"), new_window("b")
        local launcher = new_layer("overlay", "exclusive")
        close_window(a)
        return seat0:focused() == launcher
        "#,
    );
    assert!(on_layer);
}

#[test]
fn focusing_a_window_raises_it() {
    let lua = lua_with_modules();
    let name: String = eval(
        &lua,
        r#"
        require("focus")
        local a, b = new_window("a"), new_window("b")
        seat0:focus(a)
        return wm.windows:topmost().name
        "#,
    );
    assert_eq!(name, "a");
}

#[test]
fn exclusive_layer_takes_focus_and_gives_it_back() {
    let lua = lua_with_modules();
    let result: Vec<String> = lua
        .load(
            r#"
            require("focus")
            local a = new_window("a")
            local launcher = new_layer("overlay", "exclusive")
            local on_layer = seat0:focused() == launcher
            close_layer(launcher)
            return { tostring(on_layer), seat0:focused().name }
            "#,
        )
        .eval()
        .unwrap();
    assert_eq!(result, ["true", "a"]);
}

#[test]
fn bars_and_background_layers_never_take_focus() {
    let lua = lua_with_modules();
    let name: String = eval(
        &lua,
        r#"
        require("focus")
        new_window("a")
        new_layer("top", "none")
        new_layer("bottom", "exclusive")
        return seat0:focused().name
        "#,
    );
    assert_eq!(name, "a");
}

#[test]
fn clicking_decides_focus_by_what_is_under_the_pointer() {
    let lua = lua_with_modules();
    let result: Vec<String> = lua
        .load(
            r#"
            require("focus")
            local a, b = new_window("a"), new_window("b")
            local pointer = new_pointer(10, 10)
            local result = {}

            under = a
            press_button(pointer, "Pressed", Button.Left)
            table.insert(result, seat0:focused().name .. "/" .. wm.windows:topmost().name)

            under = new_layer("top", "none")
            press_button(pointer, "Pressed", Button.Left)
            table.insert(result, seat0:focused().name)

            under = nil
            press_button(pointer, "Pressed", Button.Left)
            table.insert(result, tostring(seat0:focused()))
            return result
            "#,
        )
        .eval()
        .unwrap();
    assert_eq!(result, ["a/a", "a", "nil"]);
}

#[test]
fn following_the_mouse_focuses_without_raising() {
    let lua = lua_with_modules();
    let result: Vec<String> = lua
        .load(
            r#"
            local focus = require("focus")
            local a, b = new_window("a"), new_window("b")
            local pointer = new_pointer(10, 10)
            local result = {}

            under = a
            move_pointer(pointer, 1, 1)
            table.insert(result, seat0:focused().name)

            focus.follow_mouse(true)
            move_pointer(pointer, 2, 2)
            table.insert(result, seat0:focused().name .. "/" .. wm.windows:topmost().name)

            under = new_layer("top", "none")
            move_pointer(pointer, 3, 3)
            table.insert(result, seat0:focused().name)

            under = b
            pointer.held = { Button.Left }
            move_pointer(pointer, 4, 4)
            table.insert(result, seat0:focused().name)
            return result
            "#,
        )
        .eval()
        .unwrap();
    assert_eq!(result, ["b", "a/b", "a", "a"]);
}

#[test]
fn unlocking_the_session_focuses_the_topmost_window() {
    let lua = lua_with_modules();
    let name: String = eval(
        &lua,
        r#"
        require("focus")
        local a, b = new_window("a"), new_window("b")
        seat0:focus(nil)
        wm:emit("session_unlocked")
        return seat0:focused().name
        "#,
    );
    assert_eq!(name, "b");
}

#[test]
fn only_a_fresh_activation_token_moves_focus() {
    let lua = lua_with_modules();
    let names: Vec<String> = eval(
        &lua,
        r#"
        require("focus")
        local a, b = new_window("a"), new_window("b")
        wm.windows:emit("activation_request", a, 20000)
        local after_stale = seat0:focused().name
        wm.windows:emit("activation_request", a, 500)
        return { after_stale, seat0:focused().name }
        "#,
    );
    assert_eq!(names, ["b", "a"]);
}

#[test]
fn seats_focus_different_windows_and_both_are_activated() {
    let lua = lua_with_modules();
    let result: Vec<bool> = eval(
        &lua,
        r#"
        require("focus")
        local seat1 = new_seat("seat1")
        local a, b = new_window("a"), new_window("b")
        seat0:focus(a)
        seat1:focus(b)
        return { seat0:focused() == a, seat1:focused() == b, a.activated, b.activated }
        "#,
    );
    assert_eq!(result, [true, true, true, true]);
}

#[test]
fn a_window_stays_activated_until_the_last_seat_leaves_it() {
    let lua = lua_with_modules();
    let result: Vec<bool> = eval(
        &lua,
        r#"
        require("focus")
        local seat1 = new_seat("seat1")
        local a, b = new_window("a"), new_window("b")
        seat0:focus(a)
        seat1:focus(a)
        seat1:focus(b)
        local still = a.activated
        seat0:focus(b)
        return { still, a.activated, b.activated, #a:focused_seats() == 0, #b:focused_seats() == 2 }
        "#,
    );
    assert_eq!(result, [true, false, true, true, true]);
}

#[test]
fn a_click_focuses_on_the_seat_of_the_pointer() {
    let lua = lua_with_modules();
    let result: Vec<String> = eval(
        &lua,
        r#"
        require("focus")
        local seat1 = new_seat("seat1")
        local a, b = new_window("a"), new_window("b")
        seat0:focus(a)
        seat1:focus(a)
        under = b
        press_button(new_pointer(0, 0, seat1), "Pressed", "Left")
        return { seat0:focused().name, seat1:focused().name }
        "#,
    );
    assert_eq!(result, ["a", "b"]);
}

#[test]
fn a_new_window_goes_to_the_seat_that_was_used_last() {
    let lua = lua_with_modules();
    let result: Vec<String> = eval(
        &lua,
        r#"
        require("focus")
        local seat1 = new_seat("seat1")
        local a = new_window("a")
        wm.input:emit("key", key_event("Pressed", "x", {}, 1, seat1))
        local b = new_window("b")
        return { seat0:focused().name, seat1:focused().name }
        "#,
    );
    assert_eq!(result, ["a", "b"]);
}

#[test]
fn closing_a_window_only_refocuses_the_seats_that_had_it() {
    let lua = lua_with_modules();
    let result: Vec<String> = eval(
        &lua,
        r#"
        require("focus")
        local seat1 = new_seat("seat1")
        local a, b, c = new_window("a"), new_window("b"), new_window("c")
        seat0:focus(a)
        seat1:focus(b)
        seat1:focus(nil)
        c:raise()
        close_window(a)
        return { seat0:focused().name, tostring(seat1:focused()) }
        "#,
    );
    assert_eq!(result, ["c", "nil"]);
}

#[test]
fn minimizing_a_window_moves_only_the_seats_on_it() {
    let lua = lua_with_modules();
    let result: Vec<String> = eval(
        &lua,
        r#"
        require("focus")
        local requests = require("requests")
        local seat1 = new_seat("seat1")
        local a, b, c = new_window("a"), new_window("b"), new_window("c")
        seat0:focus(a)
        seat1:focus(b)
        c:raise()
        requests.set_minimized(a, true)
        return { seat0:focused().name, seat1:focused().name }
        "#,
    );
    assert_eq!(result, ["c", "b"]);
}
