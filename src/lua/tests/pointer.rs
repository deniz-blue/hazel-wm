use super::{eval, lua_with_modules};

#[test]
fn a_constraint_activates_once_the_pointer_is_over_its_window_and_then_holds() {
    let lua = lua_with_modules();
    let result: Vec<bool> = lua
        .load(
            r#"
            require("pointer")
            local a, b = new_window("a"), new_window("b")
            local pointer = new_pointer(0, 0)
            seat0:focus(a)
            under = nil
            local constraint = new_constraint(pointer, a)
            local result = { constraint:active() }

            under = a
            move_pointer(pointer, 1, 1)
            table.insert(result, constraint:active())

            under = b
            move_pointer(pointer, 2, 2)
            table.insert(result, constraint:active())
            return result
            "#,
        )
        .eval()
        .unwrap();
    assert_eq!(result, [false, true, true]);
}

#[test]
fn a_constraint_made_under_the_pointer_activates_at_once() {
    let lua = lua_with_modules();
    let active: bool = eval(
        &lua,
        r#"
        require("pointer")
        local a = new_window("a")
        local pointer = new_pointer(0, 0)
        seat0:focus(a)
        under = a
        return new_constraint(pointer, a):active()
        "#,
    );
    assert!(active);
}

#[test]
fn dead_constraints_are_never_touched() {
    let lua = lua_with_modules();
    let active: bool = eval(
        &lua,
        r#"
        require("pointer")
        local a = new_window("a")
        local pointer = new_pointer(0, 0)
        seat0:focus(a)
        local constraint = new_constraint(pointer, a)
        constraint.is_alive = false
        under = a
        move_pointer(pointer, 1, 1)
        return constraint:active()
        "#,
    );
    assert!(!active);
}

#[test]
fn a_constraint_lets_go_when_its_window_loses_focus() {
    let lua = lua_with_modules();
    let result: Vec<bool> = lua
        .load(
            r#"
            require("pointer")
            local a, b = new_window("a"), new_window("b")
            local pointer = new_pointer(0, 0)
            seat0:focus(a)
            under = a
            local constraint = new_constraint(pointer, a)
            local result = { constraint:active() }

            seat0:focus(b)
            table.insert(result, constraint:active())
            return result
            "#,
        )
        .eval()
        .unwrap();
    assert_eq!(result, [true, false]);
}

#[test]
fn an_unfocused_window_is_never_constrained() {
    let lua = lua_with_modules();
    let active: bool = eval(
        &lua,
        r#"
        require("pointer")
        local a, b = new_window("a"), new_window("b")
        local pointer = new_pointer(0, 0)
        seat0:focus(b)
        under = a
        return new_constraint(pointer, a):active()
        "#,
    );
    assert!(!active);
}
