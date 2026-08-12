//! Minimal operating-system clipboard bridge.
//!
//! The web-facing Clipboard API is permission-gated in `vex-app`; this module
//! only performs the final Unicode text transfer to the Windows clipboard.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemClipboardError(&'static str);

impl fmt::Display for SystemClipboardError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for SystemClipboardError {}

#[cfg(target_os = "windows")]
const CF_UNICODETEXT: u32 = 13;
#[cfg(target_os = "windows")]
const GMEM_MOVEABLE: u32 = 0x0002;

#[cfg(target_os = "windows")]
#[link(name = "user32")]
extern "system" {
    fn OpenClipboard(owner: *mut std::ffi::c_void) -> i32;
    fn CloseClipboard() -> i32;
    fn EmptyClipboard() -> i32;
    fn GetClipboardData(format: u32) -> *mut std::ffi::c_void;
    fn SetClipboardData(format: u32, memory: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
}

#[cfg(target_os = "windows")]
#[link(name = "kernel32")]
extern "system" {
    fn GlobalAlloc(flags: u32, bytes: usize) -> *mut std::ffi::c_void;
    fn GlobalFree(memory: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    fn GlobalLock(memory: *mut std::ffi::c_void) -> *mut std::ffi::c_void;
    fn GlobalUnlock(memory: *mut std::ffi::c_void) -> i32;
}

#[cfg(target_os = "windows")]
struct ClipboardGuard;

#[cfg(target_os = "windows")]
impl Drop for ClipboardGuard {
    fn drop(&mut self) {
        // SAFETY: a guard is only constructed after `OpenClipboard` succeeds.
        unsafe { CloseClipboard() };
    }
}

/// Read Unicode text from the system clipboard.
#[cfg(target_os = "windows")]
pub fn read_text() -> Result<String, SystemClipboardError> {
    // SAFETY: null owner is supported by Win32; we close it through the guard.
    if unsafe { OpenClipboard(std::ptr::null_mut()) } == 0 {
        return Err(SystemClipboardError("could not open the system clipboard"));
    }
    let _guard = ClipboardGuard;
    // SAFETY: clipboard is open, and CF_UNICODETEXT is an HGLOBAL owned by the OS.
    let handle = unsafe { GetClipboardData(CF_UNICODETEXT) };
    if handle.is_null() {
        return Err(SystemClipboardError(
            "the system clipboard does not contain text",
        ));
    }
    // SAFETY: `handle` is a valid HGLOBAL while the clipboard remains open.
    let ptr = unsafe { GlobalLock(handle) as *const u16 };
    if ptr.is_null() {
        return Err(SystemClipboardError(
            "could not access system clipboard text",
        ));
    }
    // SAFETY: CF_UNICODETEXT is a NUL-terminated UTF-16 buffer by Win32 contract.
    let mut length = 0usize;
    unsafe {
        while *ptr.add(length) != 0 {
            length += 1;
        }
        let text = String::from_utf16(std::slice::from_raw_parts(ptr, length))
            .map_err(|_| SystemClipboardError("system clipboard text is not valid UTF-16"));
        GlobalUnlock(handle);
        text
    }
}

/// Write Unicode text to the system clipboard.
#[cfg(target_os = "windows")]
pub fn write_text(text: &str) -> Result<(), SystemClipboardError> {
    let utf16 = utf16z(text);
    let bytes = std::mem::size_of_val(utf16.as_slice());
    // SAFETY: Win32 allocates a movable HGLOBAL; ownership transfers on success.
    let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes) };
    if handle.is_null() {
        return Err(SystemClipboardError("could not allocate clipboard memory"));
    }
    // SAFETY: `handle` was allocated above and points at space for `bytes` bytes.
    let ptr = unsafe { GlobalLock(handle) as *mut u16 };
    if ptr.is_null() {
        // SAFETY: ownership remains with us because the allocation was not published.
        unsafe { GlobalFree(handle) };
        return Err(SystemClipboardError("could not access clipboard memory"));
    }
    // SAFETY: source and destination are valid for exactly `utf16.len()` u16 values.
    unsafe {
        std::ptr::copy_nonoverlapping(utf16.as_ptr(), ptr, utf16.len());
        GlobalUnlock(handle);
    }
    // SAFETY: null owner is supported by Win32; guard guarantees CloseClipboard.
    if unsafe { OpenClipboard(std::ptr::null_mut()) } == 0 {
        // SAFETY: ownership was not transferred because clipboard was never opened.
        unsafe { GlobalFree(handle) };
        return Err(SystemClipboardError("could not open the system clipboard"));
    }
    let _guard = ClipboardGuard;
    // SAFETY: clipboard is open. Clearing it replaces its current contents.
    if unsafe { EmptyClipboard() } == 0 {
        // SAFETY: ownership was not transferred after an EmptyClipboard failure.
        unsafe { GlobalFree(handle) };
        return Err(SystemClipboardError("could not clear the system clipboard"));
    }
    // SAFETY: on success Windows assumes ownership of `handle`.
    if unsafe { SetClipboardData(CF_UNICODETEXT, handle) }.is_null() {
        // SAFETY: failed SetClipboardData leaves ownership with this process.
        unsafe { GlobalFree(handle) };
        return Err(SystemClipboardError(
            "could not write to the system clipboard",
        ));
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn read_text() -> Result<String, SystemClipboardError> {
    Err(SystemClipboardError(
        "system clipboard is only implemented on Windows",
    ))
}

#[cfg(not(target_os = "windows"))]
pub fn write_text(_text: &str) -> Result<(), SystemClipboardError> {
    Err(SystemClipboardError(
        "system clipboard is only implemented on Windows",
    ))
}

fn utf16z(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(test)]
mod tests {
    use super::utf16z;

    #[test]
    fn utf16_text_is_null_terminated() {
        assert_eq!(utf16z("Vigo ✓"), vec![86, 105, 103, 111, 32, 10003, 0]);
    }
}
