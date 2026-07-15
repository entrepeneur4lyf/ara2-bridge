#![no_main]

use ara2_bridge_core::RestoreFilter;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let data = &data[..data.len().min(64 * 1024)];
    let mut fields = data.splitn(5, |byte| *byte == 0);
    let document_data = fields.next().is_some_and(|field| field.first() == Some(&1));
    let source_archive = String::from_utf8_lossy(fields.next().unwrap_or_default()).into_owned();
    let source_current = String::from_utf8_lossy(fields.next().unwrap_or_default()).into_owned();
    let modification_archive =
        String::from_utf8_lossy(fields.next().unwrap_or_default()).into_owned();
    let modification_current =
        String::from_utf8_lossy(fields.next().unwrap_or_default()).into_owned();

    let filter = RestoreFilter::builder()
        .document_data(document_data)
        .audio_source(source_archive, source_current)
        .audio_modification(modification_archive, modification_current)
        .build();
    if let Ok(filter) = filter {
        let ffi = filter.as_ffi();
        // SAFETY: the pinned owner retains the complete record and every nested string/array for
        // the production copy operation.
        let copied = unsafe { RestoreFilter::copy_selection_from_ffi(ffi.as_ptr()) };
        assert!(copied.is_ok());
    }
});
