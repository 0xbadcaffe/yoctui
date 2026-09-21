#[derive(Debug)]
pub struct DaemonConnection {
    stream: UnixStream,
    server_mode: bool,
    pending: Vec<u8>,
    expected_frame_len: Option<usize>,
    write_poisoned: bool,
    outgoing: Option<(Vec<u8>, usize, Instant)>,
}

impl DaemonConnection {
    pub fn connect(paths: &RuntimePaths, timeout: Duration) -> Result<Self, IpcError> {
        let deadline = Instant::now() + timeout;
        loop {
            match UnixStream::connect(&paths.socket) {
                Ok(stream) => {
                    return Ok(Self {
                        stream,
                        server_mode: false,
                        pending: Vec::new(),
                        expected_frame_len: None,
                        write_poisoned: false,
                        outgoing: None,
                    });
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::NotFound
                            | io::ErrorKind::ConnectionRefused
                            | io::ErrorKind::WouldBlock
                    ) && Instant::now() < deadline =>
                {
                    thread::sleep(CONNECT_RETRY_INTERVAL.min(timeout));
                }
                Err(source) => {
                    return Err(IpcError::Unavailable {
                        path: paths.socket.clone(),
                        source,
                    });
                }
            }
        }
    }

    pub fn set_timeout(&self, timeout: Option<Duration>) -> Result<(), IpcError> {
        self.set_read_timeout(timeout)?;
        self.set_write_timeout(timeout)?;
        Ok(())
    }

    pub fn set_read_timeout(&self, timeout: Option<Duration>) -> Result<(), IpcError> {
        self.stream.set_read_timeout(timeout)?;
        Ok(())
    }

    pub fn set_write_timeout(&self, timeout: Option<Duration>) -> Result<(), IpcError> {
        self.stream.set_write_timeout(timeout)?;
        Ok(())
    }

    /// Report whether receiving can make progress without waiting for a new
    /// peer write. Daemon client servicing uses this to avoid one blocking
    /// read timeout per attached client on every service slice.
    pub fn is_readable(&self) -> Result<bool, IpcError> {
        if self
            .expected_frame_len
            .is_some_and(|frame_len| self.pending.len() >= frame_len)
            || (self.expected_frame_len.is_none() && self.pending.len() >= 4)
        {
            return Ok(true);
        }
        let mut descriptor = libc::pollfd {
            fd: self.stream.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // SAFETY: `descriptor` points to one initialized pollfd and the stream
        // owns the descriptor for this nonblocking readiness query.
        let result = unsafe { libc::poll(&mut descriptor, 1, 0) };
        if result < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                return Ok(false);
            }
            return Err(error.into());
        }
        Ok(result > 0
            && descriptor.revents & (libc::POLLIN | libc::POLLHUP | libc::POLLERR | libc::POLLNVAL)
                != 0)
    }

    pub fn send<T: Serialize>(&mut self, message: &T) -> Result<(), IpcError> {
        let frame = encode_frame(message)?;
        self.send_encoded_frame(&frame)
    }

    /// Send one frame produced by [`encode_frame`] without serializing it
    /// again. Daemon fan-out uses this to share immutable event encoding across
    /// attached clients.
    pub fn send_encoded_frame(&mut self, frame: &[u8]) -> Result<(), IpcError> {
        if self.outgoing.is_some() {
            return Err(IpcError::Timeout("pending event write"));
        }
        if self.write_poisoned {
            return Err(IpcError::Disconnected);
        }
        if frame.len() < 4 || frame.len() > MAX_FRAME_BYTES.saturating_add(4) {
            return Err(DaemonProtocolError::InvalidLength.into());
        }
        let payload_len = u32::from_be_bytes(
            frame[..4]
                .try_into()
                .expect("the frame length guard provides four bytes"),
        ) as usize;
        if payload_len != frame.len() - 4 {
            return Err(DaemonProtocolError::InvalidLength.into());
        }
        match self.stream.write_all(frame).map_err(map_timeout) {
            Ok(()) => {}
            Err(error) if self.server_mode && is_peer_disconnect(&error) => {
                self.write_poisoned = true;
                return Ok(());
            }
            Err(error) => return Err(error),
        }
        match self.stream.flush().map_err(map_timeout) {
            Ok(()) => {}
            Err(error) if self.server_mode && is_peer_disconnect(&error) => {
                self.write_poisoned = true;
                return Ok(());
            }
            Err(error) => return Err(error),
        }
        Ok(())
    }

    pub fn send_encoded_frame_with_timeout(
        &mut self,
        frame: &[u8],
        timeout: Duration,
    ) -> Result<(), IpcError> {
        let previous = self.stream.write_timeout()?;
        self.stream.set_write_timeout(Some(timeout))?;
        let result = self.send_encoded_frame(frame);
        if !self.write_poisoned {
            self.stream.set_write_timeout(previous)?;
        }
        result
    }

    /// Queue at most one bounded event frame. The daemon must finish this
    /// frame before advancing to another event or responding to a command.
    pub fn queue_event_frame(&mut self, frame: &[u8]) -> Result<(), IpcError> {
        if self.write_poisoned {
            return Err(IpcError::Disconnected);
        }
        if self.outgoing.is_some() {
            return Err(IpcError::Timeout("pending event write"));
        }
        if frame.len() < 4
            || frame.len() > MAX_FRAME_BYTES.saturating_add(4)
            || u32::from_be_bytes(frame[..4].try_into().unwrap()) as usize != frame.len() - 4
        {
            return Err(DaemonProtocolError::InvalidLength.into());
        }
        self.outgoing = Some((frame.to_vec(), 0, Instant::now()));
        Ok(())
    }

    pub fn event_write_pending(&self) -> bool {
        self.outgoing.is_some()
    }

    /// One nonblocking write per service slice, retaining partial frame bytes.
    /// A peer that cannot finish one frame within five seconds is disconnected.
    pub fn flush_event_frame(&mut self) -> Result<bool, IpcError> {
        if self.write_poisoned {
            return Err(IpcError::Disconnected);
        }
        let Some((frame, offset, started)) = self.outgoing.as_mut() else {
            return Ok(true);
        };
        if started.elapsed() >= Duration::from_secs(5) {
            self.write_poisoned = true;
            return Err(IpcError::Timeout("event delivery"));
        }
        let remaining = &frame[*offset..];
        // SAFETY: the connection owns the descriptor and `remaining` stays
        // valid for this call. MSG_DONTWAIT does not change receive semantics.
        let written = unsafe {
            libc::send(
                self.stream.as_raw_fd(),
                remaining.as_ptr().cast(),
                remaining.len().min(64 * 1024),
                libc::MSG_DONTWAIT | libc::MSG_NOSIGNAL,
            )
        };
        if written < 0 {
            let error = io::Error::last_os_error();
            if matches!(
                error.kind(),
                io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
            ) {
                return Ok(false);
            }
            self.write_poisoned = true;
            return Err(error.into());
        }
        if written == 0 {
            self.write_poisoned = true;
            return Err(IpcError::Disconnected);
        }
        *offset += written as usize;
        if *offset == frame.len() {
            self.outgoing = None;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn receive<T: DeserializeOwned>(&mut self) -> Result<T, IpcError> {
        if self.write_poisoned {
            return Err(IpcError::Disconnected);
        }
        loop {
            if self.expected_frame_len.is_none() && self.pending.len() >= 4 {
                let payload_len = u32::from_be_bytes(
                    self.pending[..4]
                        .try_into()
                        .expect("four-byte frame prefix"),
                ) as usize;
                if payload_len > MAX_FRAME_BYTES {
                    self.pending.clear();
                    return Err(DaemonProtocolError::TooLarge.into());
                }
                self.expected_frame_len = Some(payload_len + 4);
            }
            if let Some(frame_len) = self.expected_frame_len
                && self.pending.len() >= frame_len
            {
                let frame = self.pending.drain(..frame_len).collect::<Vec<_>>();
                self.expected_frame_len = None;
                return Ok(decode_frame(&frame)?);
            }

            let mut chunk = [0_u8; 16 * 1024];
            match read_retrying_interrupts(&mut self.stream, &mut chunk).map_err(map_timeout) {
                Ok(0) => return Err(IpcError::Disconnected),
                Ok(read) => self.pending.extend_from_slice(&chunk[..read]),
                Err(error) => return Err(error),
            }
        }
    }

    pub fn peer_uid(&self) -> Result<u32, IpcError> {
        peer_uid(&self.stream)
    }
}

fn read_retrying_interrupts(reader: &mut impl Read, buffer: &mut [u8]) -> Result<usize, io::Error> {
    loop {
        match reader.read(buffer) {
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            result => return result,
        }
    }
}

fn is_peer_disconnect(error: &IpcError) -> bool {
    match error {
        IpcError::Disconnected => true,
        IpcError::Timeout(_) => true,
        IpcError::Io(source) => matches!(
            source.kind(),
            io::ErrorKind::BrokenPipe
                | io::ErrorKind::ConnectionReset
                | io::ErrorKind::NotConnected
        ),
        _ => false,
    }
}
