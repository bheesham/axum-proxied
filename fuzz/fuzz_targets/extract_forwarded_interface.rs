#![no_main]

use axum_proxied::extract::forwarded::Interface;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(value) = std::str::from_utf8(data) else {
        return ();
    };
    let _ = value.parse::<Interface>();
});
