use std::{process::Command, time::Duration};

use super::run_to_completion;

const TIMEOUT: Duration = Duration::from_secs(2);

pub const SYSTEMD_COMMAND: [&str; 3] = ["systemctl", "--user", "set-environment"];
pub const DBUS_COMMAND: [&str; 1] = ["dbus-update-activation-environment"];

pub fn is_valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn run_with_assignments(base: &[&str], variables: &[(String, String)]) -> Result<(), String> {
    let (program, fixed) = base.split_first().ok_or("empty command")?;
    let mut command = Command::new(program);
    command.args(fixed);
    command.args(variables.iter().map(|(name, value)| format!("{name}={value}")));
    let completed = run_to_completion(command, Some(TIMEOUT)).map_err(|e| format!("{program}: {e}"))?;
    if completed.report.timed_out {
        return Err(format!("{program}: no answer after {} ms", TIMEOUT.as_millis()));
    }
    match completed.report.code {
        Some(0) => Ok(()),
        _ => {
            let stderr = String::from_utf8_lossy(&completed.stderr);
            Err(format!("{program}: {}", stderr.lines().next().unwrap_or("failed")))
        }
    }
}

pub fn import_environment_with(
    systemd: &[&str],
    dbus: &[&str],
    variables: &[(String, String)],
) -> Vec<String> {
    if variables.is_empty() {
        return Vec::new();
    }
    [systemd, dbus]
        .into_iter()
        .filter_map(|base| run_with_assignments(base, variables).err())
        .collect()
}

pub fn import_environment(variables: &[(String, String)]) -> Vec<String> {
    import_environment_with(&SYSTEMD_COMMAND, &DBUS_COMMAND, variables)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars() -> Vec<(String, String)> {
        vec![("WAYLAND_DISPLAY".to_owned(), "wayland-9".to_owned()), ("DISPLAY".to_owned(), ":3".to_owned())]
    }

    fn borrowed(args: &[String]) -> Vec<&str> {
        args.iter().map(String::as_str).collect()
    }

    #[test]
    fn names_follow_the_environment_variable_rules() {
        assert!(is_valid_name("WAYLAND_DISPLAY"));
        assert!(is_valid_name("_x1"));
        assert!(!is_valid_name(""));
        assert!(!is_valid_name("1X"));
        assert!(!is_valid_name("A=B"));
        assert!(!is_valid_name("-h"));
    }

    #[test]
    fn both_commands_get_every_assignment() {
        let dir = std::env::temp_dir().join(format!("hazel-session-env-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let log = dir.join("log");
        let script = format!("echo \"$0 $*\" >> {}", log.display());
        let logging = |name: &str| {
            ["sh".to_owned(), "-c".to_owned(), script.clone(), name.to_owned()]
        };
        let (first, second) = (logging("first"), logging("second"));
        let failures = import_environment_with(&borrowed(&first), &borrowed(&second), &vars());
        assert!(failures.is_empty());
        let logged = std::fs::read_to_string(&log).unwrap();
        assert_eq!(logged, "first WAYLAND_DISPLAY=wayland-9 DISPLAY=:3\nsecond WAYLAND_DISPLAY=wayland-9 DISPLAY=:3\n");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_missing_program_and_a_failing_one_are_both_reported() {
        let failing = ["sh", "-c", "echo no bus >&2; exit 1", "x"];
        let failures = import_environment_with(&["/nonexistent/systemctl"], &failing, &vars());
        assert_eq!(failures.len(), 2);
        assert!(failures[0].starts_with("/nonexistent/systemctl"));
        assert_eq!(failures[1], "sh: no bus");
    }

    #[test]
    fn nothing_runs_without_variables() {
        assert!(import_environment_with(&["/nonexistent"], &["/nonexistent"], &[]).is_empty());
    }
}
