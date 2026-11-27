use super::lua_with_modules;

fn evaluate<T: mlua::FromLua>(code: &str) -> mlua::Result<T> {
    lua_with_modules().load(code).eval()
}

fn error_text(code: &str) -> String {
    evaluate::<mlua::Value>(code).err().map(|e| e.to_string()).unwrap_or_default()
}

#[test]
fn a_rect_cannot_have_a_negative_size() {
    assert!(error_text("return Rect(0, 0, -1, 5)").contains("negative size"));
    assert!(error_text("return Rect({ x = 0, y = 0, width = 5, height = -5 })").contains("negative size"));
}

#[test]
fn a_rect_cannot_leave_the_32_bit_range() {
    assert!(error_text("return Rect(2147483647, 0, 5, 5)").contains("coordinate range"));
}

#[test]
fn rects_are_read_from_tables_by_name_or_position() {
    let width: i32 = evaluate("return Rect({ 1, 2, 30, 40 }).width").unwrap();
    assert_eq!(width, 30);
    let height: i32 = evaluate("return Rect({ x = 1, y = 2, width = 30, height = 40 }).height").unwrap();
    assert_eq!(height, 40);
}

#[test]
fn insetting_past_the_middle_leaves_an_empty_rect() {
    let size: Vec<i32> = evaluate("local r = Rect(0, 0, 10, 10):inset(100) return { r.width, r.height }").unwrap();
    assert_eq!(size, [0, 0]);
}

#[test]
fn merging_rects_that_span_more_than_the_range_is_an_error() {
    let code = "return Rect(-2147483648, 0, 10, 10):merge(Rect(2147483600, 0, 10, 10))";
    assert!(error_text(code).contains("coordinate range"));
}

#[test]
fn points_are_read_from_tables_by_name_or_position() {
    let x: f64 = evaluate("return Point({ 3, 4 }).x").unwrap();
    assert_eq!(x, 3.0);
    let y: f64 = evaluate("return Point({ x = 3, y = 4 }).y").unwrap();
    assert_eq!(y, 4.0);
}
