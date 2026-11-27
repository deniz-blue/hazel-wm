use super::{eval, lua_with_modules};

const SETUP: &str = r#"
    require("main")
    local tiling = require("tiling")
    tiling.set_gaps(0, 0)
"#;

fn run<T: mlua::FromLua>(body: &str) -> T {
    eval(&lua_with_modules(), &format!("{SETUP}{body}"))
}

#[test]
fn windows_stay_on_their_workspaces_after_a_reload() {
    let result: Vec<String> = run(r#"
        local a = new_window("a")
        tiling.active("2")
        local b = new_window("b")
        local c = new_window("c")
        tiling.set_layout("2", "vertical")
        reload()
        tiling = require("tiling")
        local info = {}
        for _, workspace in ipairs(tiling.list()) do
            table.insert(info, workspace.name .. ":" .. workspace.layout .. ":" .. workspace.count)
        end
        table.insert(info, tiling.current())
        table.insert(info, tostring(a.visible) .. tostring(b.visible) .. tostring(c.visible))
        return info
    "#);
    assert_eq!(
        result,
        ["1:master_stack:1", "2:vertical:2", "3:monocle:0", "4:horizontal:0", "5:vertical:0", "2", "falsetruetrue"]
    );
}

#[test]
fn floating_windows_are_still_floating_after_a_reload() {
    let result: Vec<bool> = run(r#"
        local a, b = new_window("a"), new_window("b")
        require("floating").set_floating(b, true)
        reload()
        local floating = require("floating")
        return { floating.is_floating(a), floating.is_floating(b) }
    "#);
    assert_eq!(result, [false, true]);
}

#[test]
fn a_maximized_window_restores_its_old_size_after_a_reload() {
    let result: Vec<i64> = run(r#"
        local a = new_window("a")
        require("floating").set_floating(a, true)
        a.x, a.y, a.width, a.height = 10, 20, 300, 200
        require("requests").set_maximized(a, true)
        reload()
        require("requests").set_maximized(a, false)
        return { a.x, a.y, a.width, a.height }
    "#);
    assert_eq!(result, [10, 20, 300, 200]);
}

#[test]
fn windows_unknown_to_the_store_land_on_the_current_workspace() {
    let result: Vec<i64> = run(r#"
        local a = new_window("a")
        wm.store.tiling = nil
        reload()
        local info = require("tiling").list()
        return { info[1].count, a.width }
    "#);
    assert_eq!(result, [1, 1000]);
}

#[test]
fn minimized_windows_stay_minimized_after_a_reload() {
    let result: Vec<bool> = run(r#"
        local a, b = new_window("a"), new_window("b")
        require("requests").set_minimized(b, true)
        reload()
        tiling = require("tiling")
        tiling.retile()
        return { a.visible, b.visible, require("requests").is_minimized(b) }
    "#);
    assert_eq!(result, [true, false, true]);
}

#[test]
fn a_session_start_is_marked_so_autostart_runs_only_once() {
    let result: Vec<i64> = run(r#"
        local started = 0
        function spawn() started = started + 1 end
        require("autostart").once("alacritty")
        reload()
        require("autostart").once("alacritty")
        return { started }
    "#);
    assert_eq!(result, [1]);
}

#[test]
fn xwayland_satellite_is_started_once_per_session() {
    let result: Vec<i64> = run(r#"
        local started = 0
        function spawn(program) if program == "xwayland-satellite" then started = started + 1 end end
        local session = require("session")
        wm.is_reload = false
        session.start_xwayland_satellite()
        wm.is_reload = true
        session.start_xwayland_satellite()
        return { started }
    "#);
    assert_eq!(result, [1]);
}

#[test]
fn the_display_satellite_reports_reaches_the_environment_and_spawned_programs() {
    let result: Vec<String> = run(r#"
        local calls = {}
        function spawn(program, args, options) table.insert(calls, { program = program, options = options }) end
        local session = require("session")
        wm.is_reload = false
        wm.imported = {}
        session.start_xwayland_satellite()
        local satellite = calls[1].options
        satellite.on_stderr("2026-11-24T18:10:05.732Z INFO  xwayland_satellite > Connected to Xwayland on :3")
        session.spawn("alacritty", {}, { env = { COLORTERM = "truecolor" } })
        local env = calls[2].options.env
        return { wm.imported[1].DISPLAY, env.DISPLAY, env.COLORTERM }
    "#);
    assert_eq!(result, [":3", ":3", "truecolor"]);
}

#[test]
fn programs_started_before_satellite_answers_get_no_display() {
    let result: Vec<bool> = run(r#"
        local options_seen
        function spawn(_, _, options) options_seen = options end
        require("session").spawn("alacritty")
        return { options_seen.env == nil }
    "#);
    assert_eq!(result, [true]);
}
