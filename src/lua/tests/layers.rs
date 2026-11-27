use super::{eval, lua_with_modules};

const SETUP: &str = r#"
    require("main")
    local tiling = require("tiling")
    tiling.set_gaps(0, 0)
    local first = wm.outputs:get(1)
    first.box = Rect(0, 0, 1000, 800)
    first.usable = Rect(0, 0, 1000, 800)

    local function usable_from_layers(output)
        local area = output:area()
        local top, right, bottom, left = 0, 0, 0, 0
        for _, layer in ipairs(wm.layers:on_output(output.name)) do
            local zone = layer:exclusive_zone()
            if zone > 0 then
                local a, m = layer:anchor(), layer:margin()
                local edge = layer:exclusive_edge()
                local count = (a.top and 1 or 0) + (a.bottom and 1 or 0) + (a.left and 1 or 0) + (a.right and 1 or 0)
                if not edge and count == 1 then
                    edge = (a.top and "top") or (a.bottom and "bottom") or (a.left and "left") or "right"
                elseif not edge and count == 3 then
                    edge = (not a.top and "bottom") or (not a.bottom and "top") or (not a.left and "right") or "left"
                end
                if edge == "top" then top = top + zone + m.top end
                if edge == "bottom" then bottom = bottom + zone + m.bottom end
                if edge == "left" then left = left + zone + m.left end
                if edge == "right" then right = right + zone + m.right end
            end
        end
        return Rect(area.x + left, area.y + top, area.width - left - right, area.height - top - bottom)
    end
"#;

fn run<T: mlua::FromLua>(body: &str) -> T {
    eval(&lua_with_modules(), &format!("{SETUP}{body}"))
}

#[test]
fn every_property_a_client_set_can_be_read() {
    let result: Vec<String> = run(r#"
        local layer = new_layer("top", "none", {
            anchor = { top = true, left = true, right = true },
            exclusive_zone = 30,
            exclusive_edge = "top",
            margin = { top = 2, left = 4 },
            size = { width = 0, height = 30 },
        })
        local anchor, margin, size = layer:anchor(), layer:margin(), layer:requested_size()
        return {
            tostring(anchor.top), tostring(anchor.bottom), tostring(anchor.left), tostring(anchor.right),
            tostring(layer:exclusive_zone()), layer:exclusive_edge(),
            tostring(margin.top), tostring(margin.right), tostring(margin.bottom), tostring(margin.left),
            tostring(size.width), tostring(size.height),
        }
    "#);
    assert_eq!(result, ["true", "false", "true", "true", "30", "top", "2", "0", "0", "4", "0", "30"]);
}

#[test]
fn the_usable_area_is_the_area_less_the_exclusive_zones() {
    let result: Vec<bool> = run(r#"
        new_layer("top", "none", { anchor = { top = true, left = true, right = true }, exclusive_zone = 30 })
        new_layer("bottom", "none", { anchor = { left = true, top = true, bottom = true }, exclusive_zone = 50, margin = { left = 10 } })
        new_layer("top", "none", { anchor = { bottom = true, left = true, right = true }, exclusive_zone = 20, exclusive_edge = "bottom" })
        new_layer("top", "none", { anchor = { top = true }, exclusive_zone = 0 })
        new_layer("top", "none", { anchor = { top = true, bottom = true }, exclusive_zone = 40 })
        local found = usable_from_layers(first)
        first.usable = Rect(60, 30, 940, 750)
        local shortcut = first:usable_area()
        return { found.x == shortcut.x, found.y == shortcut.y, found.width == shortcut.width, found.height == shortcut.height }
    "#);
    assert_eq!(result, [true, true, true, true]);
}

#[test]
fn a_layer_changing_lays_the_bar_and_the_windows_out_again() {
    let result: Vec<i64> = run(r#"
        local window = new_window("a")
        local before = { first.shapes[1].spec.y, window.y }
        local panel = new_layer("top", "none", { anchor = { top = true, left = true, right = true }, exclusive_zone = 30 })
        first.usable = Rect(0, 30, 1000, 770)
        change_layer(panel, { exclusive_zone = 30 })
        return { before[1], before[2], first.shapes[1].spec.y, window.y }
    "#);
    assert_eq!(result, [0, 28, 30, 58]);
}

#[test]
fn a_removed_layer_gives_its_space_back_in_the_handler() {
    let result: Vec<i64> = run(r#"
        local window = new_window("a")
        local panel = new_layer("top", "none", { anchor = { top = true, left = true, right = true }, exclusive_zone = 30 })
        first.usable = Rect(0, 30, 1000, 770)
        change_layer(panel, { exclusive_zone = 30 })
        local before = { first.shapes[1].spec.y, window.y }
        first.usable = Rect(0, 0, 1000, 800)
        close_layer(panel)
        return { before[1], before[2], first.shapes[1].spec.y, window.y }
    "#);
    assert_eq!(result, [30, 58, 0, 28]);
}

#[test]
fn a_layer_on_the_left_moves_the_bar_across() {
    let result: Vec<i64> = run(r#"
        local dock = new_layer("top", "none", { anchor = { left = true, top = true, bottom = true }, exclusive_zone = 60 })
        first.usable = Rect(60, 0, 940, 800)
        change_layer(dock, { exclusive_zone = 60 })
        return { first.shapes[1].spec.x, first.shapes[1].spec.width }
    "#);
    assert_eq!(result, [60, 940]);
}
