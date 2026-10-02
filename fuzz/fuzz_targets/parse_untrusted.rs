#![no_main]

use c2pa_structured_text::extract_manifest;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &str| {
    let _ = extract_manifest(data);
});
