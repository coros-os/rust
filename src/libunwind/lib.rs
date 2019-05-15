#![no_std]
#![unstable(feature = "panic_unwind", issue = "32837")]

#![deny(rust_2018_idioms)]

#![feature(link_cfg)]
#![feature(nll)]
#![feature(staged_api)]
#![feature(unwind_attributes)]
#![feature(static_nobundle)]

#![cfg_attr(not(target_env = "msvc"), feature(libc))]

#[macro_use]
mod macros;

cfg_if! {
    if #[cfg(target_env = "msvc")] {
        // no extra unwinder support needed
    } else if #[cfg(all(target_arch = "wasm32", not(target_os = "emscripten")))] {
        // no unwinder on the system!
    } else {
        mod libunwind;
        pub use libunwind::*;
    }
}

#[cfg(target_env = "musl")]
#[link(name = "unwind", kind = "static", cfg(target_feature = "crt-static"))]
#[link(name = "gcc_s", cfg(not(target_feature = "crt-static")))]
extern {}

#[cfg(target_os = "redox")]
#[link(name = "gcc", kind = "static-nobundle", cfg(target_feature = "crt-static"))]
#[link(name = "gcc_eh", kind = "static-nobundle", cfg(target_feature = "crt-static"))]
#[link(name = "gcc_s", cfg(not(target_feature = "crt-static")))]
extern {}
