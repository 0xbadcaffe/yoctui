//! Read-only terminal graphics capability discovery.

use super::*;
use yoctui_model::HardwareGraphicsCapability;

const QUERY_TIMEOUT: Duration = Duration::from_millis(400);
const MAX_REPLY_BYTES: usize = 512;

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
    device_attributes(response).is_some_and(|parameters| {
        let mut fields = parameters.split(|byte| *byte == b';');
        // VT100-family device numbers are not capability parameters.
        let terminal = fields.next().unwrap_or_default();
        matches!(terminal, b"62" | b"63" | b"64" | b"65")
            && fields.any(|parameter| parameter == b"4")
    })
}

fn device_attributes(response: &[u8]) -> Option<&[u8]> {
    response
        .windows(3)
        .enumerate()
        .find_map(|(offset, marker)| {
            if marker != b"\x1b[?" {
                return None;
            }
            let remainder = &response[offset + 3..];
            let end = remainder.iter().position(|byte| *byte == b'c')?;
            let parameters = &remainder[..end];
            (!parameters.is_empty()
                && parameters
                    .split(|byte| *byte == b';')
                    .all(|field| !field.is_empty() && field.iter().all(u8::is_ascii_digit)))
            .then_some(parameters)
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
    read_device_attributes(input, QUERY_TIMEOUT)
}

#[cfg(unix)]
fn read_device_attributes(input: libc::c_int, timeout: Duration) -> Option<Vec<u8>> {
    let deadline = std::time::Instant::now() + timeout;
    let mut descriptor = libc::pollfd {
        fd: input,
        events: libc::POLLIN,
        revents: 0,
    };
    let mut response = Vec::with_capacity(MAX_REPLY_BYTES);
    while response.len() < MAX_REPLY_BYTES {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        let timeout_ms = remaining.as_millis().max(1).min(i32::MAX as u128) as i32;
        let ready = unsafe { libc::poll(&mut descriptor, 1, timeout_ms) };
        if ready < 0 && io::Error::last_os_error().kind() == io::ErrorKind::Interrupted {
            continue;
        }
        if ready <= 0 {
            break;
        }
        let mut buffer = [0_u8; MAX_REPLY_BYTES];
        let read = unsafe {
            libc::read(
                input,
                buffer.as_mut_ptr().cast(),
                MAX_REPLY_BYTES - response.len(),
            )
        };
        if read <= 0 {
            break;
        }
        response.extend_from_slice(&buffer[..read as usize]);
        if device_attributes(&response).is_some() {
            return Some(response);
        }
    }
    None
}

#[cfg(test)]
#[path = "tests/terminal_graphics.rs"]
mod tests;
