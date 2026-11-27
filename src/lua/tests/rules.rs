use super::{eval, lua_with_modules};

fn run<T: mlua::FromLua>(body: &str) -> T {
    eval(
        &lua_with_modules(),
        &format!(
            r#"
            require("main")
            local tiling = require("tiling")
            local floating = require("floating")
            local rules = require("rules")
            tiling.set_gaps(0, 0)
            {body}
            "#
        ),
    )
}

#[test]
fn a_rule_can_send_a_window_to_a_workspace() {
    let result: Vec<i64> = run(r#"
        rules.add({ app_id = "^chat$", workspace = "5" })
        map_window("one", "editor")
        map_window("two", "chat")
        local counts = {}
        for _, workspace in ipairs(tiling.list()) do table.insert(counts, workspace.count) end
        return counts
    "#);
    assert_eq!(result, [1, 0, 0, 0, 1]);
}

#[test]
fn a_rule_can_float_and_layer_a_window() {
    let result: Vec<String> = run(r#"
        rules.add({ title = "^popup", floating = true, layer = "above" })
        local main = map_window("main", "term")
        local popup = map_window("popup menu", "term")
        return { tostring(floating.is_floating(main)), tostring(floating.is_floating(popup)), popup:layer(), main:layer() }
    "#);
    assert_eq!(result, ["false", "true", "above", "normal"]);
}

#[test]
fn a_rule_only_applies_once_the_window_has_mapped() {
    let result: Vec<bool> = run(r#"
        rules.add({ app_id = "^chat$", floating = true })
        local window = new_window("chat")
        window.app = "chat"
        local before = floating.is_floating(window)
        window.is_mapped = true
        wm.windows:emit("mapped", window)
        return { before, floating.is_floating(window) }
    "#);
    assert_eq!(result, [false, true]);
}

#[test]
fn windows_get_a_focus_colored_border_once_mapped() {
    let result: Vec<String> = run(r#"
        local a = map_window("a", "x")
        local b = map_window("b", "x")
        return { a.border.color, b.border.color, tostring(a.border.width) }
    "#);
    assert_eq!(result, ["#4c566a", "#88c0d0", "2"]);
}

#[test]
fn fullscreen_windows_lose_their_border() {
    let result: Vec<bool> = run(r#"
        local a = map_window("a", "x")
        local had = a.border ~= nil
        require("requests").set_fullscreen(a, true)
        return { had, a.border == nil }
    "#);
    assert_eq!(result, [true, true]);
}

#[test]
fn gaps_separate_windows_and_the_screen_edge() {
    let result: Vec<i64> = run(r#"
        tiling.set_gaps(10, 20)
        local a, b = new_window("a"), new_window("b")
        return { a.x, a.y, a.width, a.height, b.x, b.y, b.width, b.height }
    "#);
    assert_eq!(result, [20, 48, 572, 732, 602, 48, 378, 732]);
}

#[test]
fn a_window_asks_for_a_specific_output_when_going_fullscreen() {
    let result: Vec<i64> = run(r#"
        local a = new_window("a")
        local elsewhere = { area = function() return Rect(1000, 0, 640, 480) end }
        wm.windows:emit("fullscreen_request", a, elsewhere)
        return { a.x, a.width }
    "#);
    assert_eq!(result, [1000, 640]);
}

#[test]
fn minimizing_hides_the_window_and_restoring_brings_back_the_last_one() {
    let result: Vec<bool> = run(r#"
        local requests = require("requests")
        local a, b = new_window("a"), new_window("b")
        wm.windows:emit("minimize_request", a)
        wm.windows:emit("minimize_request", b)
        local hidden = { a.visible, b.visible }
        requests.restore_last_minimized()
        return { hidden[1], hidden[2], a.visible, b.visible }
    "#);
    assert_eq!(result, [false, false, false, true]);
}

#[test]
fn unminimize_request_shows_the_window_again() {
    let result: Vec<bool> = run(r#"
        local a = new_window("a")
        wm.windows:emit("minimize_request", a)
        local hidden = a.visible
        wm.windows:emit("unminimize_request", a)
        return { hidden, a.visible }
    "#);
    assert_eq!(result, [false, true]);
}
