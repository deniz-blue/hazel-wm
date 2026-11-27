mod lines;
mod run;
mod session_environment;

use std::{
    io::{self, ErrorKind, Read},
    os::unix::process::{CommandExt, ExitStatusExt},
    process::{Child, Command, ExitStatus, Stdio},
    time::{Duration, Instant},
};

use rustix::{
    fs::{OFlags, fcntl_getfl, fcntl_setfl},
    process::{Pid, Signal, kill_process_group},
};

pub use lines::LineBuffer;
pub use run::{Completed, run_to_completion};
pub use session_environment::{import_environment, is_valid_name};

const GRACE_BEFORE_KILL: Duration = Duration::from_secs(1);

#[derive(Clone, Copy, Default)]
pub struct Watch {
    pub stdout: bool,
    pub stderr: bool,
    pub exit: bool,
    pub timeout: Option<Duration>,
    pub survives_reload: bool,
}

impl Watch {
    fn is_observed(&self) -> bool {
        self.stdout || self.stderr || self.exit
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct ExitReport {
    pub code: Option<i32>,
    pub signal: Option<i32>,
    pub timed_out: bool,
}

#[derive(Debug, PartialEq)]
pub enum ProcessEvent {
    Stdout(u32, Vec<u8>),
    Stderr(u32, Vec<u8>),
    Exited(u32, ExitReport),
}

struct Running {
    pid: u32,
    child: Child,
    watch: Watch,
    stdout: Option<std::process::ChildStdout>,
    stderr: Option<std::process::ChildStderr>,
    deadline: Option<Instant>,
    terminated_at: Option<Instant>,
}

#[derive(Default)]
pub struct Processes {
    running: Vec<Running>,
}

fn make_nonblocking(fd: &impl std::os::fd::AsFd) -> io::Result<()> {
    let flags = fcntl_getfl(fd)?;
    fcntl_setfl(fd, flags | OFlags::NONBLOCK)?;
    Ok(())
}

fn pipe_of<R: std::os::fd::AsFd>(reader: Option<R>) -> io::Result<Option<R>> {
    if let Some(reader) = &reader {
        make_nonblocking(reader)?;
    }
    Ok(reader)
}

fn signal_group(pid: u32, signal: Signal) {
    if let Some(pid) = i32::try_from(pid).ok().and_then(Pid::from_raw) {
        let _ = kill_process_group(pid, signal);
    }
}

fn drain<R: Read>(
    pipe: &mut Option<R>,
    make_event: impl Fn(Vec<u8>) -> ProcessEvent,
    emit: bool,
    events: &mut Vec<ProcessEvent>,
) {
    let Some(reader) = pipe else { return };
    let mut chunk = [0u8; 4096];
    loop {
        match reader.read(&mut chunk) {
            Ok(0) => {
                *pipe = None;
                return;
            }
            Ok(count) if emit => events.push(make_event(chunk[..count].to_vec())),
            Ok(_) => {}
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            Err(error) if error.kind() == ErrorKind::WouldBlock => return,
            Err(_) => {
                *pipe = None;
                return;
            }
        }
    }
}

fn report_of(status: ExitStatus, timed_out: bool) -> ExitReport {
    ExitReport { code: status.code(), signal: status.signal(), timed_out }
}

impl Running {
    fn read_pipes(&mut self, events: &mut Vec<ProcessEvent>) {
        let pid = self.pid;
        drain(&mut self.stdout, |line| ProcessEvent::Stdout(pid, line), self.watch.stdout, events);
        drain(&mut self.stderr, |line| ProcessEvent::Stderr(pid, line), self.watch.stderr, events);
    }

    fn enforce_timeout(&mut self, now: Instant) {
        match (self.deadline, self.terminated_at) {
            (Some(deadline), None) if now >= deadline => {
                signal_group(self.pid, Signal::TERM);
                self.terminated_at = Some(now);
            }
            (_, Some(since)) if now.duration_since(since) >= GRACE_BEFORE_KILL => {
                signal_group(self.pid, Signal::KILL);
            }
            _ => {}
        }
    }

    fn poll(&mut self, now: Instant, events: &mut Vec<ProcessEvent>) -> bool {
        self.read_pipes(events);
        self.enforce_timeout(now);
        let Ok(Some(status)) = self.child.try_wait() else {
            return true;
        };
        self.read_pipes(events);
        if self.watch.exit {
            events.push(ProcessEvent::Exited(self.pid, report_of(status, self.terminated_at.is_some())));
        }
        false
    }
}

impl Processes {
    pub fn spawn(&mut self, mut command: Command, watch: Watch) -> io::Result<u32> {
        let piped = |wanted: bool| if wanted { Stdio::piped() } else { Stdio::null() };
        let mut child = command
            .stdin(Stdio::null())
            .stdout(piped(watch.stdout))
            .stderr(piped(watch.stderr))
            .process_group(0)
            .spawn()?;
        let pid = child.id();
        let (stdout, stderr) = (pipe_of(child.stdout.take()), pipe_of(child.stderr.take()));
        let (stdout, stderr) = match (stdout, stderr) {
            (Ok(stdout), Ok(stderr)) => (stdout, stderr),
            (Err(error), _) | (_, Err(error)) => {
                signal_group(pid, Signal::KILL);
                let _ = child.wait();
                return Err(error);
            }
        };
        let deadline = watch.timeout.map(|timeout| Instant::now() + timeout);
        self.running.push(Running { pid, child, watch, stdout, stderr, deadline, terminated_at: None });
        Ok(pid)
    }

    pub fn poll(&mut self, now: Instant) -> Vec<ProcessEvent> {
        let mut events = Vec::new();
        self.running.retain_mut(|running| running.poll(now, &mut events));
        events
    }

    pub fn signal(&self, pid: u32, signal: Signal) -> bool {
        let known = self.running.iter().any(|running| running.pid == pid);
        if known {
            signal_group(pid, signal);
        }
        known
    }

    pub fn observed_pids(&self) -> Vec<u32> {
        self.running
            .iter()
            .filter(|running| running.watch.is_observed())
            .map(|running| running.pid)
            .collect()
    }

    pub fn drop_observers_on_reload(&mut self, pids: &[u32]) {
        for running in self.running.iter_mut().filter(|running| pids.contains(&running.pid)) {
            if !running.watch.survives_reload {
                signal_group(running.pid, Signal::TERM);
            }
            running.watch = Watch::default();
        }
    }

    pub fn terminate_all(&mut self, grace: Duration) {
        for running in &self.running {
            signal_group(running.pid, Signal::TERM);
        }
        let end = Instant::now() + grace;
        while Instant::now() < end && self.running.iter_mut().any(|r| matches!(r.child.try_wait(), Ok(None))) {
            std::thread::sleep(Duration::from_millis(10));
        }
        for running in &mut self.running {
            if matches!(running.child.try_wait(), Ok(None)) {
                signal_group(running.pid, Signal::KILL);
                let _ = running.child.wait();
            }
        }
        self.running.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shell(script: &str) -> Command {
        let mut command = Command::new("sh");
        command.args(["-c", script]);
        command
    }

    fn collect_until_exit(processes: &mut Processes) -> Vec<ProcessEvent> {
        let mut events = Vec::new();
        for _ in 0..1000 {
            events.extend(processes.poll(Instant::now()));
            if events.iter().any(|event| matches!(event, ProcessEvent::Exited(..))) {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        events
    }

    #[test]
    fn output_and_exit_are_reported() {
        let mut processes = Processes::default();
        let watch = Watch { stdout: true, stderr: true, exit: true, ..Watch::default() };
        let pid = processes.spawn(shell("printf out; printf err >&2; exit 4"), watch).unwrap();
        let events = collect_until_exit(&mut processes);
        assert!(events.contains(&ProcessEvent::Stdout(pid, b"out".to_vec())));
        assert!(events.contains(&ProcessEvent::Stderr(pid, b"err".to_vec())));
        let exit = ExitReport { code: Some(4), signal: None, timed_out: false };
        assert_eq!(events.last(), Some(&ProcessEvent::Exited(pid, exit)));
    }

    #[test]
    fn a_reload_ends_observed_processes_except_the_ones_that_survive_it() {
        let alive = |pid: u32| rustix::process::test_kill_process_group(Pid::from_raw(pid as i32).unwrap()).is_ok();
        let mut processes = Processes::default();
        let observed = Watch { exit: true, ..Watch::default() };
        let ends = processes.spawn(shell("exec sleep 30"), observed).unwrap();
        let survives = processes.spawn(shell("exec sleep 30"), Watch { survives_reload: true, ..observed }).unwrap();
        let pids = processes.observed_pids();
        processes.drop_observers_on_reload(&pids);
        std::thread::sleep(Duration::from_millis(100));
        processes.poll(Instant::now());
        assert!(!alive(ends));
        assert!(alive(survives));
        assert!(processes.observed_pids().is_empty());
        processes.terminate_all(Duration::from_millis(200));
    }

    #[test]
    fn an_unobserved_process_reports_nothing() {
        let mut processes = Processes::default();
        processes.spawn(shell("echo hidden"), Watch::default()).unwrap();
        std::thread::sleep(Duration::from_millis(100));
        assert!(processes.poll(Instant::now()).is_empty());
        assert!(processes.running.is_empty());
    }

    #[test]
    fn a_timeout_terminates_the_process() {
        let mut processes = Processes::default();
        let watch = Watch { exit: true, timeout: Some(Duration::from_millis(50)), ..Watch::default() };
        let pid = processes.spawn(shell("exec sleep 30"), watch).unwrap();
        let events = collect_until_exit(&mut processes);
        let exit = ExitReport { code: None, signal: Some(15), timed_out: true };
        assert_eq!(events.last(), Some(&ProcessEvent::Exited(pid, exit)));
    }

    #[test]
    fn terminating_all_reaches_grandchildren() {
        let mut processes = Processes::default();
        let pid = processes.spawn(shell("sleep 30 & echo $! > /dev/null; wait"), Watch::default()).unwrap();
        std::thread::sleep(Duration::from_millis(100));
        processes.terminate_all(Duration::from_millis(500));
        let group_alive = Pid::from_raw(pid as i32).is_some_and(|pid| rustix::process::test_kill_process_group(pid).is_ok());
        assert!(!group_alive);
    }
}
