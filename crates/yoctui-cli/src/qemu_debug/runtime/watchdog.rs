//! Async-signal-safe child-side guardian: even a SIGKILL of the PTY helper closes
//! its private lifetime pipe and stops only the newly spawned runqemu group.
use std::{
    io,
    os::fd::{AsRawFd, FromRawFd, OwnedFd},
};
use tokio::process::Command;

pub(super) fn install(command: &mut Command) -> io::Result<(OwnedFd, OwnedFd)> {
    #[cfg(not(target_os = "linux"))]
    {
        let _ = command;
        return Err(io::Error::other(
            "Managed QEMU cleanup currently requires Linux",
        ));
    }
    #[cfg(target_os = "linux")]
    {
        let mut descriptors = [-1; 2];
        if unsafe { libc::pipe2(descriptors.as_mut_ptr(), libc::O_CLOEXEC) } != 0 {
            return Err(io::Error::last_os_error());
        }
        let read = unsafe { OwnedFd::from_raw_fd(descriptors[0]) };
        let write = unsafe { OwnedFd::from_raw_fd(descriptors[1]) };
        let input = read.as_raw_fd();
        let output = write.as_raw_fd();
        let maximum = unsafe { libc::sysconf(libc::_SC_OPEN_MAX) }.max(1024);
        // No allocation, locks or Rust runtime work after fork and before exec.
        unsafe {
            command.pre_exec(move || {
                let leader = libc::getpid();
                let guardian = libc::fork();
                if guardian < 0 {
                    return Err(io::Error::last_os_error());
                }
                if guardian == 0 {
                    libc::close(output);
                    if libc::setsid() < 0 {
                        libc::_exit(1);
                    }
                    libc::close(libc::STDIN_FILENO);
                    libc::close(libc::STDOUT_FILENO);
                    libc::close(libc::STDERR_FILENO);
                    // Also close Tokio's exec-error pipe: retaining it would
                    // prevent spawn() from completing. Keep only the lifetime reader.
                    let first =
                        libc::syscall(libc::SYS_close_range, 3_u32, (input - 1) as u32, 0_u32);
                    let second =
                        libc::syscall(libc::SYS_close_range, (input + 1) as u32, u32::MAX, 0_u32);
                    if first < 0 || second < 0 {
                        for descriptor in 3..maximum {
                            if descriptor != i64::from(input) {
                                libc::close(descriptor as i32);
                            }
                        }
                    }
                    let mut byte = 0_u8;
                    loop {
                        let result = libc::read(input, (&mut byte as *mut u8).cast(), 1);
                        if result >= 0 {
                            break;
                        }
                        if *libc::__errno_location() != libc::EINTR {
                            break;
                        }
                    }
                    libc::kill(-leader, libc::SIGKILL);
                    libc::_exit(0);
                }
                libc::close(input);
                libc::close(output);
                Ok(())
            });
        }
        Ok((read, write))
    }
}
