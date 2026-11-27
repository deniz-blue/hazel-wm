use std::{
    fs::File,
    io::{self, Read, Write},
    mem,
    os::fd::OwnedFd,
    sync::Arc,
};

use rustix::fs::{OFlags, fcntl_setfl};
use smithay::reexports::calloop::{Interest, LoopHandle, Mode, PostAction, generic::Generic};

const CHUNK: usize = 64 * 1024;

enum ReadProgress {
    Waiting,
    Finished(io::Result<()>),
}

fn set_nonblocking(fd: &OwnedFd) -> io::Result<()> {
    fcntl_setfl(fd, OFlags::NONBLOCK).map_err(io::Error::from)
}

fn read_available(mut source: impl Read, collected: &mut Vec<u8>, limit: usize) -> ReadProgress {
    let mut chunk = [0; CHUNK];
    loop {
        match source.read(&mut chunk) {
            Ok(0) => return ReadProgress::Finished(Ok(())),
            Ok(count) if collected.len() + count > limit => {
                return ReadProgress::Finished(Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("more than {limit} bytes"),
                )));
            }
            Ok(count) => collected.extend_from_slice(&chunk[..count]),
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => return ReadProgress::Waiting,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return ReadProgress::Finished(Err(e)),
        }
    }
}

pub fn read_limited<D: 'static>(
    handle: &LoopHandle<'static, D>,
    fd: OwnedFd,
    limit: usize,
    finished: impl FnOnce(&mut D, io::Result<Vec<u8>>) + 'static,
) -> io::Result<()> {
    set_nonblocking(&fd)?;
    let mut collected = Vec::new();
    let mut finished = Some(finished);
    handle
        .insert_source(
            Generic::new(File::from(fd), Interest::READ, Mode::Level),
            move |_, file, data| {
                let ReadProgress::Finished(result) = read_available(&**file, &mut collected, limit) else {
                    return Ok(PostAction::Continue);
                };
                if let Some(finished) = finished.take() {
                    finished(data, result.map(|()| mem::take(&mut collected)));
                }
                Ok(PostAction::Remove)
            },
        )
        .map_err(|e| io::Error::other(e.error))?;
    Ok(())
}

fn write_available(mut target: impl Write, bytes: &[u8], written: &mut usize) -> io::Result<bool> {
    while *written < bytes.len() {
        match target.write(&bytes[*written..]) {
            Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
            Ok(count) => *written += count,
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(false),
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(true)
}

pub fn write_all<D: 'static>(
    handle: &LoopHandle<'static, D>,
    fd: OwnedFd,
    bytes: Arc<[u8]>,
) -> io::Result<()> {
    set_nonblocking(&fd)?;
    let mut written = 0;
    handle
        .insert_source(
            Generic::new(File::from(fd), Interest::WRITE, Mode::Level),
            move |_, file, _| match write_available(&**file, &bytes, &mut written) {
                Ok(false) => Ok(PostAction::Continue),
                Ok(true) | Err(_) => Ok(PostAction::Remove),
            },
        )
        .map_err(|e| io::Error::other(e.error))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{io::pipe, time::Duration};

    use smithay::reexports::calloop::EventLoop;

    use super::*;

    #[derive(Default)]
    struct Received(Option<io::Result<Vec<u8>>>);

    fn run_until_received(event_loop: &mut EventLoop<'static, Received>, received: &mut Received) {
        for _ in 0..200 {
            if received.0.is_some() {
                return;
            }
            event_loop.dispatch(Duration::from_millis(10), received).expect("dispatch");
        }
    }

    fn transfer(bytes: Vec<u8>, limit: usize) -> io::Result<Vec<u8>> {
        let mut event_loop: EventLoop<'static, Received> = EventLoop::try_new().expect("event loop");
        let handle = event_loop.handle();
        let (reader, writer) = pipe().expect("pipe");
        read_limited(&handle, reader.into(), limit, |received: &mut Received, result| {
            received.0 = Some(result);
        })
        .expect("read");
        write_all(&handle, writer.into(), bytes.into()).expect("write");
        let mut received = Received::default();
        run_until_received(&mut event_loop, &mut received);
        received.0.expect("the transfer finished")
    }

    #[test]
    fn a_transfer_larger_than_the_pipe_buffer_arrives_whole() {
        let bytes: Vec<u8> = (0..500_000).map(|index| (index % 251) as u8).collect();
        assert_eq!(transfer(bytes.clone(), 1 << 20).expect("whole"), bytes);
    }

    #[test]
    fn an_empty_transfer_arrives_empty() {
        assert_eq!(transfer(Vec::new(), 10).expect("empty"), Vec::<u8>::new());
    }

    #[test]
    fn a_transfer_over_the_limit_is_an_error() {
        let error = transfer(vec![7; 1000], 999).expect_err("too large");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    }
}
