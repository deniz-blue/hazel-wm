use super::{eval, lua_with_modules};

const SETUP: &str = r#"
    require("main")
    local tiling = require("tiling")
    local second = new_output("mock-2", 1000, 0, 800, 600)
    local first = wm.outputs:get(1)
    local function texts(output)
        local list = {}
        for _, shape in ipairs(output.shapes) do
            if shape.spec.kind == "text" and shape.spec.visible ~= false then table.insert(list, shape.spec.text) end
        end
        return table.concat(list, ",")
    end
"#;

fn run<T: mlua::FromLua>(body: &str) -> T {
    eval(&lua_with_modules(), &format!("{SETUP}{body}"))
}

#[test]
fn every_output_has_a_bar_listing_the_workspaces() {
    let result: Vec<String> = run(r#"return { texts(first), texts(second) }"#);
    assert_eq!(result, ["1,2,3,4,5", "1,2,3,4,5"]);
}

#[test]
fn a_bar_shows_which_workspace_its_output_shows() {
    let result: Vec<String> = run(r#"
        local function fills(output)
            local list = {}
            for _, shape in ipairs(output.shapes) do
                if shape.spec.kind == "rect" and shape.spec.radius then
                    table.insert(list, shape.spec.fill and "filled" or "outlined")
                end
            end
            return table.concat(list, ",")
        end
        return { fills(first), fills(second) }
    "#);
    assert_eq!(result, ["filled,outlined,outlined,outlined,outlined", "outlined,filled,outlined,outlined,outlined"]);
}

#[test]
fn the_focused_windows_title_is_on_the_bar_of_the_output_holding_it() {
    let result: Vec<String> = run(r#"
        local a = new_window("editor")
        seat0:focus(a)
        local on_first = texts(first):match("editor") ~= nil
        local on_second = texts(second):match("editor") ~= nil
        return { tostring(on_first), tostring(on_second) }
    "#);
    assert_eq!(result, ["true", "false"]);
}

#[test]
fn a_bar_that_did_not_change_is_not_redrawn() {
    let result: Vec<i64> = run(r#"
        local before = #first.shapes
        local shape = first.shapes[#first.shapes]
        tiling.retile()
        tiling.retile()
        wm.windows:emit("focus", nil, seat0)
        return { before == #first.shapes and 1 or 0, shape.updates, shape == first.shapes[#first.shapes] and 1 or 0 }
    "#);
    assert_eq!(result, [1, 0, 1]);
}

#[test]
fn only_what_changed_is_sent_when_the_title_changes() {
    let result: Vec<i64> = run(r#"
        local a = new_window("one")
        seat0:focus(a)
        local count = #first.shapes
        a.name = "two"
        wm.windows:emit("title_changed", a)
        return { count == #first.shapes and 1 or 0, texts(first):match("two") and 1 or 0 }
    "#);
    assert_eq!(result, [1, 1]);
}

#[test]
fn a_removed_output_loses_its_bar_state_and_the_others_update() {
    let result: Vec<String> = run(r#"
        remove_output(second)
        return { texts(first), tostring(#second.shapes > 0) }
    "#);
    assert_eq!(result[0], "1,2,3,4,5");
}

#[test]
fn a_bar_follows_a_resolution_change() {
    let width: i64 = run(r#"
        first.box.width = 1280
        first.usable.width = 1280
        wm.outputs:emit("changed", first)
        return first.shapes[1].spec.width
    "#);
    assert_eq!(width, 1280);
}

#[test]
fn the_title_moves_to_the_surviving_bar_when_the_focused_windows_output_goes() {
    let result: Vec<String> = run(r#"
        move_pointer(new_pointer(0, 0), 1100, 100)
        local a = new_window("editor")
        seat0:focus(a)
        local before = texts(second):match("editor") ~= nil
        remove_output(second)
        return { tostring(before), tostring(texts(first):match("editor") ~= nil) }
    "#);
    assert_eq!(result, ["true", "true"]);
}
