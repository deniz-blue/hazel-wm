use miette::miette;
use mlua::Error;

pub trait LuaErrorExt<T> {
    fn into_miette(self) -> miette::Result<T>;
}

impl<T> LuaErrorExt<T> for Result<T, Error> {
    fn into_miette(self) -> miette::Result<T> {
        self.map_err(|err| miette!(err.to_string()))
    }
}
