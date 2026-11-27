use mlua::{Lua, Table, Value};

const MAX_DEPTH: usize = 32;

#[derive(Debug, Clone, PartialEq)]
pub enum StoreValue {
    Boolean(bool),
    Integer(i64),
    Number(f64),
    String(Vec<u8>),
    Table(Vec<(StoreValue, StoreValue)>),
}

impl StoreValue {
    pub fn from_lua(value: &Value, path: &str, depth: usize) -> mlua::Result<Option<Self>> {
        Ok(Some(match value {
            Value::Nil => return Ok(None),
            Value::Boolean(value) => Self::Boolean(*value),
            Value::Integer(value) => Self::Integer(*value),
            Value::Number(value) => Self::Number(*value),
            Value::String(value) => Self::String(value.as_bytes().to_vec()),
            Value::Table(table) if depth < MAX_DEPTH => {
                Self::Table(Self::entries_of(table, path, depth)?)
            }
            Value::Table(_) => {
                return Err(unsupported(path, "a table nested more than 32 levels deep"));
            }
            other => return Err(unsupported(path, other.type_name())),
        }))
    }

    fn entries_of(
        table: &Table,
        path: &str,
        depth: usize,
    ) -> mlua::Result<Vec<(StoreValue, StoreValue)>> {
        let mut entries = Vec::new();
        for pair in table.pairs::<Value, Value>() {
            let (key, value) = pair?;
            let child_path = format!("{path}.{}", describe_key(&key));
            let Some(key) = Self::from_lua(&key, &child_path, depth + 1)? else {
                continue;
            };
            if let Some(value) = Self::from_lua(&value, &child_path, depth + 1)? {
                entries.push((key, value));
            }
        }
        Ok(entries)
    }

    pub fn into_lua(self, lua: &Lua) -> mlua::Result<Value> {
        Ok(match self {
            Self::Boolean(value) => Value::Boolean(value),
            Self::Integer(value) => Value::Integer(value),
            Self::Number(value) => Value::Number(value),
            Self::String(bytes) => Value::String(lua.create_string(&bytes)?),
            Self::Table(entries) => Value::Table(Self::table_from(entries, lua)?),
        })
    }

    pub fn table_from(entries: Vec<(StoreValue, StoreValue)>, lua: &Lua) -> mlua::Result<Table> {
        let table = lua.create_table()?;
        for (key, value) in entries {
            table.set(key.into_lua(lua)?, value.into_lua(lua)?)?;
        }
        Ok(table)
    }
}

pub fn snapshot(table: &Table) -> mlua::Result<Vec<(StoreValue, StoreValue)>> {
    StoreValue::entries_of(table, "wm.store", 0)
}

fn describe_key(key: &Value) -> String {
    match key {
        Value::String(text) => text.to_string_lossy(),
        Value::Integer(number) => number.to_string(),
        other => other.type_name().to_owned(),
    }
}

fn unsupported(path: &str, found: &str) -> mlua::Error {
    mlua::Error::external(format!(
        "{path} is {found}; wm.store keeps booleans, numbers, strings and tables of those"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(lua: &Lua, code: &str) -> Table {
        let table: Table = lua.load(code).eval().unwrap();
        let entries = snapshot(&table).unwrap();
        StoreValue::table_from(entries, lua).unwrap()
    }

    #[test]
    fn nested_tables_survive_into_another_state() {
        let source = Lua::new();
        let table: Table = source
            .load(r#"return { current = "2", order = { "1", "2" }, windows = { [7] = true }, ratio = 0.5 }"#)
            .eval()
            .unwrap();
        let entries = snapshot(&table).unwrap();

        let target = Lua::new();
        let copy = StoreValue::table_from(entries, &target).unwrap();
        target.globals().set("copy", copy).unwrap();
        let ok: bool = target
            .load(r#"return copy.current == "2" and copy.order[2] == "2" and copy.windows[7] == true and copy.ratio == 0.5"#)
            .eval()
            .unwrap();
        assert!(ok);
    }

    #[test]
    fn strings_keep_bytes_that_are_not_utf8() {
        let lua = Lua::new();
        let copy = roundtrip(&lua, r#"return { raw = "\xff\x00\xfe" }"#);
        let raw: mlua::String = copy.get("raw").unwrap();
        assert_eq!(&*raw.as_bytes(), [0xff, 0x00, 0xfe]);
    }

    #[test]
    fn functions_are_rejected_with_their_path() {
        let lua = Lua::new();
        let table: Table = lua.load("return { tiling = { hook = print } }").eval().unwrap();
        let message = snapshot(&table).unwrap_err().to_string();
        assert!(message.contains("wm.store.tiling.hook"), "{message}");
        assert!(message.contains("function"), "{message}");
    }

    #[test]
    fn cycles_are_rejected_instead_of_looping() {
        let lua = Lua::new();
        let table: Table = lua.load("local t = {} t.self = t return t").eval().unwrap();
        assert!(snapshot(&table).is_err());
    }
}
