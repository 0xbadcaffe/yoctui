//! Read-only terminal graphics capability discovery.

use super::*;
use yoctui_model::HardwareGraphicsCapability;

const QUERY_TIMEOUT_MS: i32 = 120;

pub(crate) fn detect_hardware_graphics_capability() -> HardwareGraphicsCapability {
    match env::var("YOCTUI_TERMINAL_GRAPHICS").ok().as_deref() {
        Some("sixel") => return HardwareGraphicsCapability::Sixel,
        Some("none") => return HardwareGraphicsCapability::Unavailable,
        _ => {}
    }
    #[cfg(unix)]
    if query_primary_device_attributes()
        .as_deref()
        .is_some_and(primary_device_attributes_support_sixel)
    {
        return HardwareGraphicsCapability::Sixel;
    }
    HardwareGraphicsCapability::Unavailable
}

pub(crate) fn primary_device_attributes_support_sixel(response: &[u8]) -> bool {
    response.windows(3).enumerate().any(|(offset, marker)| {
        marker == b"\x1b[?"
            && response[offset + 3..]
                .split(|byte| *byte == b'c')
                .next()
                .is_some_and(|parameters| {
                    parameters
                        .split(|byte| *byte == b';')
                        .any(|parameter| parameter == b"4")
                })
    })
}

#[cfg(unix)]
fn query_primary_device_attributes() -> Option<Vec<u8>> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return None;
    }
    let input = libc::STDIN_FILENO;
    let mut original = std::mem::MaybeUninit::<libc::termios>::uninit();
    if unsafe { libc::tcgetattr(input, original.as_mut_ptr()) } != 0 {
        return None;
    }
    let original = unsafe { original.assume_init() };
    let mut raw = original;
    unsafe { libc::cfmakeraw(&mut raw) };
    if unsafe { libc::tcsetattr(input, libc::TCSANOW, &raw) } != 0 {
        return None;
    }
    let response = query_primary_device_attributes_raw(input);
    let _ = unsafe { libc::tcsetattr(input, libc::TCSANOW, &original) };
    response
}

#[cfg(unix)]
fn query_primary_device_attributes_raw(input: libc::c_int) -> Option<Vec<u8>> {
    io::stdout().write_all(b"\x1b[c").ok()?;
    io::stdout().flush().ok()?;
    let mut descriptor = libc::pollfd {
        fd: input,
        events: libc::POLLIN,
        revents: 0,
    };
    if unsafe { libc::poll(&mut descriptor, 1, QUERY_TIMEOUT_MS) } <= 0 {
        return None;
    }
    let mut buffer = [0_u8; 512];
    let read = unsafe { libc::read(input, buffer.as_mut_ptr().cast(), buffer.len()) };
    (read > 0).then(|| buffer[..read as usize].to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_device_attributes_require_exact_sixel_parameter() {
        assert!(primary_device_attributes_support_sixel(b"\x1b[?65;1;4;9c"));
        assert!(!primary_device_attributes_support_sixel(b"\x1b[?65;1;9c"));
        assert!(!primary_device_attributes_support_sixel(b"text 4"));
        assert!(!primary_device_attributes_support_sixel(b"\x1b[?64;14;9c"));
    }
}
