use std::{
    ffi::OsStr,
    os::unix::ffi::OsStrExt,
    path::PathBuf,
    process::Command,
    rc::Rc,
    time::Duration,
};

use mlua::{FromLua, Function, Lua, Table, Value};
use rustix::process::Signal;

use crate::{
    core::{
        GlobalHazel,
        processes::{Watch, run_to_completion},
    },
    lua::api::wm_processes::{Callbacks, WmProcesses, exit_table},
};

#[derive(Default)]
pub struct ProcessOptions {
    pub env: Vec<(Vec<u8>, Vec<u8>)>,
    pub cwd: Option<PathBuf>,
    pub timeout: Option<Duration>,
    pub on_stdout: Option<Function>,
    pub on_stderr: Option<Function>,
    pub on_exit: Option<Function>,
    pub survives_reload: bool,
}

impl FromLua for ProcessOptions {
    fn from_lua(value: Value, _: &Lua) -> mlua::Result<Self> {
        let table = match value {
            Value::Nil => return Ok(Self::default()),
            Value::Table(table) => table,
            other => {
                return Err(mlua::Error::FromLuaConversionError {
                    from: other.type_name(),
                    to: "ProcessOptions".to_owned(),
                    message: Some("expected a table of options".to_owned()),
                });
            }
        };
        let mut env = Vec::new();
        if let Some(vars) = table.get::<Option<Table>>("env")? {
            for pair in vars.pairs::<mlua::String, mlua::String>() {
                let (name, value) = pair?;
                env.push((name.as_bytes().to_vec(), value.as_bytes().to_vec()));
            }
        }
        let cwd = table
            .get::<Option<mlua::String>>("cwd")?
            .map(|cwd| PathBuf::from(OsStr::from_bytes(&cwd.as_bytes())));
        let timeout = table
            .get::<Option<f64>>("timeout")?
            .filter(|ms| *ms >= 0.0)
            .map(|ms| {
                Duration::try_from_secs_f64(ms / 1000.0)
                    .map_err(|_| mlua::Error::external("timeout is too large"))
            })
            .transpose()?;
        Ok(Self {
            env,
            cwd,
            timeout,
            on_stdout: table.get("on_stdout")?,
            on_stderr: table.get("on_stderr")?,
            on_exit: table.get("on_exit")?,
            survives_reload: table.get::<Option<bool>>("survives_reload")?.unwrap_or(false),
        })
    }
}

impl ProcessOptions {
    fn watch(&self) -> Watch {
        Watch {
            stdout: self.on_stdout.is_some(),
            stderr: self.on_stderr.is_some(),
            exit: self.on_exit.is_some(),
            timeout: self.timeout,
            survives_reload: self.survives_reload,
        }
    }

    fn callbacks(&self, lua: &Lua) -> mlua::Result<Callbacks> {
        let key = |callback: &Option<Function>| callback.clone().map(|f| lua.create_registry_value(f)).transpose();
        Ok(Callbacks {
            on_stdout: key(&self.on_stdout)?,
            on_stderr: key(&self.on_stderr)?,
            on_exit: key(&self.on_exit)?,
        })
    }
}

pub fn command(program: &mlua::String, args: &[mlua::String], options: &ProcessOptions) -> Command {
    let mut command = Command::new(OsStr::from_bytes(&program.as_bytes()));
    command.args(args.iter().map(|arg| OsStr::from_bytes(&arg.as_bytes()).to_owned()));
    command.envs(
        options
            .env
            .iter()
            .map(|(name, value)| (OsStr::from_bytes(name), OsStr::from_bytes(value))),
    );
    if let Some(cwd) = &options.cwd {
        command.current_dir(cwd);
    }
    command
}

pub fn register(lua: &Lua, processes: Rc<WmProcesses>) -> mlua::Result<()> {
    let globals = lua.globals();

    globals.set(
        "spawn",
        lua.create_function(
            move |lua, (program, args, options): (mlua::String, Option<Vec<mlua::String>>, ProcessOptions)| {
                let command = command(&program, &args.unwrap_or_default(), &options);
                let callbacks = options.callbacks(lua)?;
                GlobalHazel::try_with(|hazel| {
                    let pid = hazel.processes.spawn(command, options.watch()).map_err(mlua::Error::external)?;
                    processes.register(pid, callbacks);
                    Ok(pid)
                })
            },
        )?,
    )?;

    globals.set("exec", lua.create_function(exec)?)?;
    globals.set("kill", lua.create_function(kill)?)?;
    Ok(())
}

fn kill(_: &Lua, (pid, signal): (u32, Option<String>)) -> mlua::Result<bool> {
    let signal = match signal.as_deref() {
        None | Some("term") => Signal::TERM,
        Some("kill") => Signal::KILL,
        Some("int") => Signal::INT,
        Some("hup") => Signal::HUP,
        Some(other) => return Err(mlua::Error::external(format!("unknown signal {other:?}, expected term, kill, int or hup"))),
    };
    GlobalHazel::try_with(|hazel| Ok(hazel.processes.signal(pid, signal)))
}

fn exec(
    lua: &Lua,
    (program, args, options): (mlua::String, Option<Vec<mlua::String>>, ProcessOptions),
) -> mlua::Result<Table> {
    let command = command(&program, &args.unwrap_or_default(), &options);
    let completed = run_to_completion(command, options.timeout).map_err(mlua::Error::external)?;
    let result = exit_table(lua, completed.report)?;
    result.set("stdout", lua.create_string(&completed.stdout)?)?;
    result.set("stderr", lua.create_string(&completed.stderr)?)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lua_with_exec() -> Lua {
        let lua = Lua::new();
        lua.globals()
            .set("exec", lua.create_function(exec).unwrap())
            .unwrap();
        lua
    }

    #[test]
    fn exec_keeps_bytes_that_are_not_utf8() {
        let lua = lua_with_exec();
        let bytes: Vec<u8> = lua
            .load(r#"return exec("printf", { "\\303\\251\\377" }).stdout"#)
            .eval::<mlua::String>()
            .unwrap()
            .as_bytes()
            .to_vec();
        assert_eq!(bytes, [0xc3, 0xa9, 0xff]);
    }

    #[test]
    fn exec_passes_env_and_cwd() {
        let lua = lua_with_exec();
        let out: String = lua
            .load(r#"return exec("sh", { "-c", "echo $GREETING; pwd" }, { env = { GREETING = "hi" }, cwd = "/tmp" }).stdout"#)
            .eval()
            .unwrap();
        assert_eq!(out, "hi\n/tmp\n");
    }

    #[test]
    fn exec_reports_the_exit_status() {
        let lua = lua_with_exec();
        let status: i64 = lua.load(r#"return exec("sh", { "-c", "exit 3" }).status"#).eval().unwrap();
        assert_eq!(status, 3);
    }

    #[test]
    fn exec_stops_a_process_that_outlives_its_timeout() {
        let lua = lua_with_exec();
        let (timed_out, status, signal): (bool, Option<i64>, Option<i64>) = lua
            .load(r#"local r = exec("sleep", { "30" }, { timeout = 50 }); return r.timed_out, r.status, r.signal"#)
            .eval()
            .unwrap();
        assert_eq!((timed_out, status, signal), (true, None, Some(15)));
    }

    #[test]
    fn a_timeout_that_cannot_be_a_duration_is_an_error() {
        let lua = Lua::new();
        let options = |code: &str| lua.load(code).eval::<ProcessOptions>();
        assert!(options("return { timeout = math.huge }").is_err());
        assert!(options("return { timeout = 1e300 }").is_err());
        assert!(options("return { timeout = 0/0 }").is_ok());
        assert!(options("return { timeout = 1500 }").is_ok());
    }
}
