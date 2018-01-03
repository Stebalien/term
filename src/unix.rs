extern crate libc;

use std::os::unix::io::RawFd;
use std::io;
use std::mem;
use Dims;
use Result;

/// Get the window size from a file descriptor (0 for stdout)
///
/// Option is returned as there is no distinction between errors (all ENOSYS)
pub fn win_size(fd: RawFd) -> Result<libc::winsize> {
    unsafe {
        let ws: libc::winsize = mem::uninitialized();
        if libc::ioctl(fd, libc::TIOCGWINSZ, &ws) == 0 {
            Ok(ws)
        } else {
            Err(io::Error::last_os_error().into())
        }
    }
}

impl From<libc::winsize> for Dims {
    fn from(sz: libc::winsize) -> Dims {
        Dims {
            rows: sz.ws_row as u16,
            columns: sz.ws_col as u16,
            pixel_width: Some(sz.ws_xpixel as u32),
            pixel_height: Some(sz.ws_ypixel as u32),
        }
    }
}
