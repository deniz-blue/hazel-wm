use super::{eval, lua_with_modules};

const SETUP: &str = r#"
    require("main")
    local tiling = require("tiling")
    tiling.set_gaps(0, 0)
    local second = new_output("mock-2", 1000, 0, 800, 600)
    local first = wm.outputs:get(1)
    local function names()
        local list = {}
        for _, info in ipairs(tiling.list()) do
            table.insert(list, info.name .. "@" .. tostring(info.output))
        end
        return table.concat(list, " ")
    end
"#;

fn run<T: mlua::FromLua>(body: &str) -> T {
    eval(&lua_with_modules(), &format!("{SETUP}{body}"))
}

#[test]
fn each_output_shows_its_own_workspace() {
    let shown: String = run("return names()");
    assert_eq!(shown, "1@mock-1 2@mock-2 3@nil 4@nil 5@nil");
}

#[test]
fn a_new_window_is_laid_out_in_the_area_of_the_output_showing_its_workspace() {
    let geometry: Vec<i64> = run(r#"
        local a = new_window("a")
        move_pointer(new_pointer(0, 0), 1100, 100)
        local b = new_window("b")
        return { a.x, a.y, a.width, b.x, b.y, b.width, b.height }
    "#);
    assert_eq!(geometry, [0, 28, 1000, 1000, 28, 800, 572]);
}

#[test]
fn windows_on_the_second_output_leave_the_first_alone() {
    let result: Vec<bool> = run(r#"
        local a = new_window("a")
        move_pointer(new_pointer(0, 0), 1100, 100)
        local b = new_window("b")
        tiling.active("3")
        return { a.visible, b.visible, tiling.current(second) == "3", tiling.current(first) == "1" }
    "#);
    assert_eq!(result, [true, false, true, true]);
}

#[test]
fn switching_on_one_output_leaves_the_other() {
    let shown: String = run(r#"
        tiling.active("4", first)
        tiling.active("5", second)
        return names()
    "#);
    assert_eq!(shown, "1@nil 2@nil 3@nil 4@mock-1 5@mock-2");
}

#[test]
fn showing_a_workspace_another_output_shows_swaps_the_two() {
    let shown: String = run(r#"
        tiling.active("2", first)
        return names()
    "#);
    assert_eq!(shown, "1@mock-2 2@mock-1 3@nil 4@nil 5@nil");
}

#[test]
fn a_swap_carries_floating_windows_to_the_other_output() {
    let position: Vec<i64> = run(r#"
        local f = new_window("f")
        require("floating").set_floating(f, true)
        f.x, f.y = 100, 200
        tiling.active("1", second)
        return { f.x, f.y, tiling.current(second) == "1" and 1 or 0 }
    "#);
    assert_eq!(position, [1100, 200, 1]);
}

#[test]
fn cycling_skips_workspaces_other_outputs_show() {
    let shown: String = run(r#"
        move_pointer(new_pointer(0, 0), 100, 100)
        tiling.cycle_workspace(1)
        return names()
    "#);
    assert_eq!(shown, "1@nil 2@mock-2 3@mock-1 4@nil 5@nil");
}

#[test]
fn a_new_output_gets_a_workspace_nobody_shows() {
    let shown: String = run(r#"
        new_output("mock-3", 1800, 0, 800, 600)
        return names()
    "#);
    assert_eq!(shown, "1@mock-1 2@mock-2 3@mock-3 4@nil 5@nil");
}

#[test]
fn an_output_added_when_every_workspace_is_shown_gets_a_new_workspace() {
    let shown: String = run(r#"
        for i = 3, 6 do new_output("mock-" .. i, 800 * i, 0, 800, 600) end
        return names()
    "#);
    assert_eq!(shown, "1@mock-1 2@mock-2 3@mock-3 4@mock-4 5@mock-5 6@mock-6");
}

#[test]
fn a_removed_outputs_windows_move_to_the_workspace_that_is_still_shown() {
    let result: Vec<String> = run(r#"
        local a = new_window("a")
        move_pointer(new_pointer(0, 0), 1100, 100)
        local b = new_window("b")
        local c = new_window("c")
        remove_output(second)
        local info = tiling.list()
        return { tostring(info[1].count), tostring(info[2].count), tostring(b.visible and c.visible), tostring(b.x < 1000), tostring(tiling.workspace_of(b)) }
    "#);
    assert_eq!(result, ["3", "0", "true", "true", "1"]);
}

#[test]
fn a_removed_outputs_floating_window_keeps_its_place_on_the_surviving_output() {
    let position: Vec<i64> = run(r#"
        move_pointer(new_pointer(0, 0), 1100, 100)
        local f = new_window("f")
        require("floating").set_floating(f, true)
        f.x, f.y = 1300, 50
        remove_output(second)
        return { f.x, f.y }
    "#);
    assert_eq!(position, [300, 50]);
}

#[test]
fn a_fullscreen_window_is_refitted_when_it_changes_output() {
    let result: Vec<i64> = run(r#"
        local a = new_window("a")
        require("requests").set_fullscreen(a, true)
        tiling.move_window(a, "2")
        return { a.x, a.width, a.height }
    "#);
    assert_eq!(result, [1000, 800, 600]);
}

#[test]
fn losing_the_last_output_hides_its_windows_and_the_next_output_shows_them_again() {
    let result: Vec<bool> = run(r#"
        local a = new_window("a")
        remove_output(second)
        remove_output(first)
        local hidden = not a.visible
        new_output("mock-9", 0, 0, 1000, 800)
        return { hidden, a.visible }
    "#);
    assert_eq!(result, [true, true]);
}

#[test]
fn outputs_keep_their_workspaces_through_a_reload() {
    let shown: String = run(r#"
        tiling.active("4", second)
        reload()
        tiling = require("tiling")
        return names()
    "#);
    assert_eq!(shown, "1@mock-1 2@nil 3@nil 4@mock-2 5@nil");
}

#[test]
fn the_active_output_follows_the_pointer_and_focus() {
    let result: Vec<String> = run(r#"
        local outputs = require("outputs")
        local pointer = new_pointer(0, 0)
        local before = outputs.active().name
        move_pointer(pointer, 1100, 100)
        local after_pointer = outputs.active().name
        local a = new_window("a")
        a.x = 10
        seat0:focus(a)
        return { before, after_pointer, outputs.active().name }
    "#);
    assert_eq!(result, ["mock-1", "mock-2", "mock-1"]);
}

#[test]
fn the_space_a_bar_reserves_comes_off_the_top_of_the_usable_area() {
    let area: Vec<i64> = run(r#"
        local usable = require("outputs").usable_area(second)
        return { usable.y, usable.height }
    "#);
    assert_eq!(area, [28, 572]);
}
