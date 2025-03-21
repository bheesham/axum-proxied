#![no_main]

use axum_proxied::parser;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = parser::parse(data);
});
