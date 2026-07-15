/// Reads one byte from a caller-validated address.
///
/// # Safety
///
/// `pointer` must be non-null, aligned, and readable for one initialized byte.
pub unsafe fn undocumented_read(pointer: *const u8) -> u8 {
    unsafe { pointer.read() }
}
