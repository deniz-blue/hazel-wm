use mlua::Error;

pub type BoxError = Box<dyn std::error::Error + 'static>;
pub type BoxResult<T> = std::result::Result<T, BoxError>;

pub trait LuaErrorExt<T> {
    fn into_box(self) -> BoxResult<T>;
}

impl<T> LuaErrorExt<T> for std::result::Result<T, Error> {
    fn into_box(self) -> BoxResult<T> {
        self.map_err(|err| Box::new(err) as BoxError)
    }
}
