use super::{eval, lua_with_modules};

#[test]
fn fullscreen_fills_the_output_and_restores_the_old_geometry() {
    let lua = lua_with_modules();
    let result: Vec<i64> = lua
        .load(
            r#"
            local requests = require("requests")
            local a = new_window("a")
            a:set_position(Point(100, 50))
            a:set_size({ width = 400, height = 300 })
            requests.set_fullscreen(a, true)
            local filled = { a.x, a.y, a.width, a.height }
            requests.set_fullscreen(a, false)
            return { filled[3], filled[4], a.x, a.y, a.width, a.height }
            "#,
        )
        .eval()
        .unwrap();
    assert_eq!(result, [1000, 840, 100, 50, 400, 300]);
}

#[test]
fn maximize_stays_inside_the_usable_area() {
    let lua = lua_with_modules();
    let height: i64 = eval(
        &lua,
        r#"
        local requests = require("requests")
        local a = new_window("a")
        requests.set_maximized(a, true)
        return a.height
        "#,
    );
    assert_eq!(height, 800);
}

#[test]
fn leaving_fullscreen_while_maximized_goes_back_to_maximized() {
    let lua = lua_with_modules();
    let height: i64 = eval(
        &lua,
        r#"
        local requests = require("requests")
        local a = new_window("a")
        requests.set_maximized(a, true)
        requests.set_fullscreen(a, true)
        requests.set_fullscreen(a, false)
        return a.height
        "#,
    );
    assert_eq!(height, 800);
}

#[test]
fn client_requests_are_answered_with_the_same_placement() {
    let lua = lua_with_modules();
    let result: Vec<bool> = lua
        .load(
            r#"
            require("requests")
            local a = new_window("a")
            wm.windows:emit("fullscreen_request", a)
            local on = a:states():has("fullscreen") and a.height == 840
            wm.windows:emit("unfullscreen_request", a)
            return { on, not a:states():has("fullscreen") }
            "#,
        )
        .eval()
        .unwrap();
    assert_eq!(result, [true, true]);
}

#[test]
fn clients_are_given_server_side_decorations() {
    let lua = lua_with_modules();
    let mode: String = eval(
        &lua,
        r#"
        require("requests")
        local a = new_window("a")
        wm.windows:emit("decoration_request", a)
        return a.decoration
        "#,
    );
    assert_eq!(mode, "server");
}
