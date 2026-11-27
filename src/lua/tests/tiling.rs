use super::{eval, lua_with_modules};

#[test]
fn master_stack_gives_master_sixty_percent() {
    let lua = lua_with_modules();
    let geometry: Vec<i64> = lua
        .load(
            r#"
            require("tiling")
            local a, b, c = new_window("a"), new_window("b"), new_window("c")
            return { a.x, a.width, a.height, b.x, b.y, b.width, b.height, c.y }
            "#,
        )
        .eval()
        .unwrap();
    assert_eq!(geometry, [0, 600, 800, 600, 0, 400, 400, 400]);
}

#[test]
fn switching_workspace_hides_the_others() {
    let lua = lua_with_modules();
    let visibility: Vec<bool> = lua
        .load(
            r#"
            local tiling = require("tiling")
            local first = new_window("first")
            tiling.active("2")
            local second = new_window("second")
            local while_on_two = { first.visible, second.visible }
            tiling.active("1")
            return { while_on_two[1], while_on_two[2], first.visible, second.visible }
            "#,
        )
        .eval()
        .unwrap();
    assert_eq!(visibility, [false, true, true, false]);
}

#[test]
fn closing_a_window_retiles_the_rest() {
    let lua = lua_with_modules();
    let width: i64 = eval(
        &lua,
        r#"
        require("tiling")
        local a, b = new_window("a"), new_window("b")
        close_window(b)
        return a.width
        "#,
    );
    assert_eq!(width, 1000);
}

#[test]
fn fullscreen_windows_are_left_out_of_the_layout() {
    let lua = lua_with_modules();
    let widths: Vec<i64> = lua
        .load(
            r#"
            local tiling = require("tiling")
            local a, b = new_window("a"), new_window("b")
            a:states():set("fullscreen", true)
            a.width = 1000
            tiling.retile()
            return { a.width, b.width }
            "#,
        )
        .eval()
        .unwrap();
    assert_eq!(widths, [1000, 1000]);
}

#[test]
fn moving_focused_window_to_another_workspace() {
    let lua = lua_with_modules();
    let result: Vec<String> = lua
        .load(
            r#"
            local tiling = require("tiling")
            local a, b = new_window("a"), new_window("b")
            seat0:focus(b)
            tiling.move_focused_to("2")
            local info = tiling.list()
            return { tostring(info[1].count), tostring(info[2].count), tostring(b.visible) }
            "#,
        )
        .eval()
        .unwrap();
    assert_eq!(result, ["1", "1", "false"]);
}

#[test]
fn focus_next_wraps_around_the_workspace() {
    let lua = lua_with_modules();
    let names: Vec<String> = lua
        .load(
            r#"
            local tiling = require("tiling")
            local a, b = new_window("a"), new_window("b")
            seat0:focus(b)
            tiling.focus_next(1)
            local first = seat0:focused().name
            tiling.focus_next(-1)
            return { first, seat0:focused().name }
            "#,
        )
        .eval()
        .unwrap();
    assert_eq!(names, ["a", "b"]);
}

#[test]
fn returning_to_a_workspace_refocuses_the_last_focused_window() {
    let lua = lua_with_modules();
    let name: String = eval(
        &lua,
        r#"
        local tiling = require("tiling")
        local a, b = new_window("a"), new_window("b")
        seat0:focus(a)
        tiling.active("2")
        tiling.active("1")
        return seat0:focused().name
        "#,
    );
    assert_eq!(name, "a");
}
