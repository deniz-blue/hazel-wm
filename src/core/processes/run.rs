use std::{
    io,
    process::Command,
    time::{Duration, Instant},
};

use super::{ExitReport, ProcessEvent, Processes, Watch};

const POLL_INTERVAL: Duration = Duration::from_millis(2);

pub struct Completed {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub report: ExitReport,
}

pub fn run_to_completion(command: Command, timeout: Option<Duration>) -> io::Result<Completed> {
    let mut processes = Processes::default();
    let watch = Watch { stdout: true, stderr: true, exit: true, timeout, ..Watch::default() };
    processes.spawn(command, watch)?;
    let mut completed = Completed { stdout: Vec::new(), stderr: Vec::new(), report: ExitReport::default() };
    loop {
        for event in processes.poll(Instant::now()) {
            match event {
                ProcessEvent::Stdout(_, chunk) => completed.stdout.extend(chunk),
                ProcessEvent::Stderr(_, chunk) => completed.stderr.extend(chunk),
                ProcessEvent::Exited(_, report) => {
                    completed.report = report;
                    return Ok(completed);
                }
            }
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}
