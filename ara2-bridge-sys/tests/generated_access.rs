#[test]
fn unaligned_accessors_copy_without_borrowing_fields() {
    let mut bytes = [0_u8; 16];
    // SAFETY: the array is writable for a u64 at byte offset 1.
    unsafe {
        ara2_bridge_sys::access::write_field(bytes.as_mut_ptr(), 1, 0x0102_0304_0506_0708_u64)
    };
    // SAFETY: the same initialized bytes remain readable.
    let value = unsafe { ara2_bridge_sys::access::read_field::<u64>(bytes.as_ptr(), 1) };
    assert_eq!(value, 0x0102_0304_0506_0708_u64);
}
