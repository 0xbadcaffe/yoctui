//! Drain BitBake output without allowing its console to draw over ncurses.
use std::{
    collections::VecDeque,
    io::{self, Read},
    os::fd::AsRawFd,
    process::Child,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

pub(super) const LIMIT: usize = 32 * 1024;

pub(super) struct Diagnostics {
    stop: Arc<AtomicBool>,
    reader: Option<JoinHandle<Vec<u8>>>,
}

impl Diagnostics {
    pub(super) fn start(child: &mut Child) -> io::Result<Self> {
        let stdout = child.stdout.take().expect("piped BitBake stdout");
        let stderr = child.stderr.take().expect("piped BitBake stderr");
        nonblocking(&stdout)?;
        nonblocking(&stderr)?;
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let reader = thread::spawn(move || {
            let mut streams: [Box<dyn Read>; 2] = [Box::new(stdout), Box::new(stderr)];
            let mut tails = [
                VecDeque::with_capacity(LIMIT / 2),
                VecDeque::with_capacity(LIMIT / 2),
            ];
            let mut buffer = [0; 4096];
            loop {
                let finish = stopping.load(Ordering::Acquire);
                for (stream, tail) in streams.iter_mut().zip(&mut tails) {
                    // A noisy child cannot prevent shutdown or starve stderr.
                    for _ in 0..16 {
                        match stream.read(&mut buffer) {
                            Ok(0) | Err(_) => break,
                            Ok(count) => {
                                for byte in &buffer[..count] {
                                    if tail.len() == LIMIT / 2 {
                                        tail.pop_front();
                                    }
                                    tail.push_back(*byte);
                                }
                            }
                        }
                    }
                }
                if finish {
                    break;
                }
                thread::sleep(Duration::from_millis(10));
            }
            tails.into_iter().flatten().collect()
        });
        Ok(Self {
            stop,
            reader: Some(reader),
        })
    }

    pub(super) fn finish(&mut self) -> Vec<u8> {
        self.stop.store(true, Ordering::Release);
        self.reader
            .take()
            .and_then(|reader| reader.join().ok())
            .unwrap_or_default()
    }

    pub(super) fn report(mut self) {
        let bytes = self.finish();
        if !bytes.is_empty() {
            eprintln!(
                "BitBake output (last {} bytes):\n{}",
                bytes.len(),
                String::from_utf8_lossy(&bytes)
            );
        }
    }
}

impl Drop for Diagnostics {
    fn drop(&mut self) {
        self.finish();
    }
}

fn nonblocking(stream: &impl AsRawFd) -> io::Result<()> {
    // SAFETY: the descriptor is borrowed from a live owned pipe; fcntl keeps ownership.
    let result = unsafe {
        let flags = libc::fcntl(stream.as_raw_fd(), libc::F_GETFL);
        if flags < 0 {
            return Err(io::Error::last_os_error());
        }
        libc::fcntl(stream.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK)
    };
    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}
