use std::{
    io::{self, Read, Write},
    os::unix::{
        io::{AsRawFd, RawFd},
        net::UnixStream,
    },
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Instant,
};

use super::COSMETIC_ACTIVITY_MIN_INTERVAL;

#[derive(Debug, Clone)]
pub(super) struct ActivityNotificationSender {
    writer: Arc<UnixStream>,
    pending: Arc<AtomicBool>,
    epoch: Arc<Instant>,
    last_cosmetic_signal_micros: Arc<AtomicU64>,
}
impl ActivityNotificationSender {
    pub(super) fn signal(&self) {
        if self.pending.swap(true, Ordering::AcqRel) {
            return;
        }
        let mut writer = self.writer.as_ref();
        if let Err(error) = writer.write(&[1])
            && error.kind() != io::ErrorKind::WouldBlock
        {
            self.pending.store(false, Ordering::Release);
        }
    }

    pub(super) fn signal_batched(&self) {
        let elapsed = self.epoch.elapsed().as_micros().min(u64::MAX as u128) as u64;
        let minimum = COSMETIC_ACTIVITY_MIN_INTERVAL
            .as_micros()
            .min(u64::MAX as u128) as u64;
        let mut previous = self.last_cosmetic_signal_micros.load(Ordering::Acquire);
        loop {
            if previous != u64::MAX && elapsed.saturating_sub(previous) < minimum {
                return;
            }
            match self.last_cosmetic_signal_micros.compare_exchange_weak(
                previous,
                elapsed,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => break,
                Err(current) => previous = current,
            }
        }
        self.signal();
    }
}

#[derive(Debug)]
pub(super) struct ActivityNotification {
    reader: UnixStream,
    pub(super) sender: ActivityNotificationSender,
}

impl ActivityNotification {
    pub(super) fn new() -> io::Result<Self> {
        let (reader, writer) = UnixStream::pair()?;
        reader.set_nonblocking(true)?;
        writer.set_nonblocking(true)?;
        Ok(Self {
            reader,
            sender: ActivityNotificationSender {
                writer: Arc::new(writer),
                pending: Arc::new(AtomicBool::new(false)),
                epoch: Arc::new(Instant::now()),
                last_cosmetic_signal_micros: Arc::new(AtomicU64::new(u64::MAX)),
            },
        })
    }

    pub(super) fn raw_fd(&self) -> RawFd {
        self.reader.as_raw_fd()
    }

    pub(super) fn consume(&self) {
        self.sender.pending.store(false, Ordering::Release);
        let mut reader = &self.reader;
        let mut bytes = [0_u8; 64];
        loop {
            match reader.read(&mut bytes) {
                Ok(0) | Err(_) => break,
                Ok(_) => {}
            }
        }
    }
}

impl super::DaemonBitBakeSupervisor {
    pub fn notification_fd(&self) -> Option<RawFd> {
        self.activity.as_ref().map(ActivityNotification::raw_fd)
    }

    pub fn consume_notification(&self) {
        if let Some(activity) = &self.activity {
            activity.consume();
        }
    }
}
