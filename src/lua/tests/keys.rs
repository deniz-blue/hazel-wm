use super::{eval, lua_with_modules};

#[test]
fn key_spec_splits_modifiers_from_key() {
    let lua = lua_with_modules();
    let ok: bool = eval(
        &lua,
        r#"
        local keys = require("keys")
        local binding = keys.parse("Super+shift+t")
        return binding.key == "t" and binding.modifiers.logo and binding.modifiers.shift
            and not binding.modifiers.ctrl
        "#,
    );
    assert!(ok);
}

#[test]
fn key_spec_with_unknown_modifier_is_an_error() {
    let lua = lua_with_modules();
    let result = lua
        .load(r#"require("keys").parse("hyper+t")"#)
        .exec()
        .unwrap_err();
    assert!(result.to_string().contains("unknown modifier 'hyper'"));
}

#[test]
fn binding_needs_exactly_its_modifiers() {
    let lua = lua_with_modules();
    let results: Vec<bool> = lua
        .load(
            r#"
            local keys = require("keys")
            local calls = 0
            keys.bind("super+t", function() calls = calls + 1 end)
            local exact = keys.handle(key_event("Pressed", "t", { logo = true }))
            local extra = keys.handle(key_event("Pressed", "t", { logo = true, shift = true }))
            local missing = keys.handle(key_event("Pressed", "t", {}))
            local other = keys.handle(key_event("Pressed", "w", { logo = true }))
            return { exact, extra, missing, other, calls == 1 }
            "#,
        )
        .eval()
        .unwrap();
    assert_eq!(results, [true, false, false, false, true]);
}

#[test]
fn handled_press_swallows_its_release() {
    let lua = lua_with_modules();
    let results: Vec<bool> = lua
        .load(
            r#"
            local keys = require("keys")
            keys.bind("super+t", function() end)
            keys.handle(key_event("Pressed", "t", { logo = true }, 28))
            local release = key_event("Released", "t", {}, 28)
            local unrelated = key_event("Released", "w", {}, 17)
            wm.input:emit("key", release)
            wm.input:emit("key", unrelated)
            return { release.prevented == true, unrelated.prevented == true }
            "#,
        )
        .eval()
        .unwrap();
    assert_eq!(results, [true, false]);
}

#[test]
fn locked_binding_runs_only_while_locked_and_normal_bindings_only_while_unlocked() {
    let lua = lua_with_modules();
    let results: Vec<bool> = lua
        .load(
            r#"
            local keys = require("keys")
            local locked_calls, normal_calls = 0, 0
            keys.bind_locked("XF86AudioMute", function() locked_calls = locked_calls + 1 end)
            keys.bind("super+t", function() normal_calls = normal_calls + 1 end)
            wm.is_session_locked = true
            local mute = key_event("Pressed", "XF86AudioMute", {}, 113)
            local terminal = key_event("Pressed", "t", { logo = true }, 28)
            wm.input:emit("key", mute)
            wm.input:emit("key", terminal)
            wm.is_session_locked = false
            local unlocked_mute = key_event("Pressed", "XF86AudioMute", {}, 113)
            wm.input:emit("key", unlocked_mute)
            return {
                mute.prevented == true,
                terminal.prevented == true,
                unlocked_mute.prevented == true,
                locked_calls == 1,
                normal_calls == 0,
            }
            "#,
        )
        .eval()
        .unwrap();
    assert_eq!(results, [true, false, false, true, true]);
}

#[test]
fn locked_press_swallows_its_release() {
    let lua = lua_with_modules();
    let results: Vec<bool> = lua
        .load(
            r#"
            local keys = require("keys")
            keys.bind_locked("XF86AudioMute", function() end)
            wm.is_session_locked = true
            wm.input:emit("key", key_event("Pressed", "XF86AudioMute", {}, 113))
            wm.is_session_locked = false
            local release = key_event("Released", "XF86AudioMute", {}, 113)
            wm.input:emit("key", release)
            return { release.prevented == true }
            "#,
        )
        .eval()
        .unwrap();
    assert_eq!(results, [true]);
}

#[test]
fn default_config_binds_volume_keys_while_locked_and_unlocked() {
    let lua = lua_with_modules();
    let results: Vec<bool> = lua
        .load(
            r#"
            require("main")
            local spawned = {}
            spawn = function(program, args) table.insert(spawned, program .. " " .. args[#args - 1] .. " " .. args[#args]) end
            wm.is_session_locked = true
            local locked = key_event("Pressed", "XF86AudioLowerVolume", {}, 114)
            wm.input:emit("key", locked)
            wm.is_session_locked = false
            local unlocked = key_event("Pressed", "XF86AudioLowerVolume", {}, 114)
            wm.input:emit("key", unlocked)
            return { locked.prevented == true, unlocked.prevented == true, #spawned == 2 }
            "#,
        )
        .eval()
        .unwrap();
    assert_eq!(results, [true, true, true]);
}

#[test]
fn pointer_handlers_of_the_default_config_stay_out_of_the_way_while_locked() {
    let lua = lua_with_modules();
    let prevented: bool = lua
        .load(
            r#"
            require("main")
            local floating = require("floating")
            local window = new_window("a")
            local pointer = new_pointer(300, 300)
            wm.is_session_locked = true
            local event = { state = "Pressed", button = Button.Left, modifiers = { logo = true }, pointer = pointer }
            function event:prevent_default() self.prevented = true end
            wm.input:emit("pointer_button", event)
            return event.prevented == true
            "#,
        )
        .eval()
        .unwrap();
    assert!(!prevented);
}

#[test]
fn default_config_quits_on_its_chord_also_while_locked_and_grants_shortcut_inhibitors() {
    let lua = lua_with_modules();
    let results: Vec<bool> = lua
        .load(
            r#"
            require("main")
            local quits = 0
            function wm:quit() quits = quits + 1 end
            local chord = { logo = true, ctrl = true, shift = true, alt = true }
            wm.is_session_locked = true
            wm.input:emit("key", key_event("Pressed", "q", chord, 24))
            wm.is_session_locked = false
            wm.input:emit("key", key_event("Pressed", "q", chord, 24))
            local active = false
            local inhibitor = { activate = function() active = true end }
            wm.input:emit("shortcuts_inhibit_request", inhibitor)
            return { quits == 2, active }
            "#,
        )
        .eval()
        .unwrap();
    assert_eq!(results, [true, true]);
}
