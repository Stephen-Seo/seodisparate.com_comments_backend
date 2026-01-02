// ISC License
//
// Copyright (c) 2025-2026 Stephen Seo
//
// Permission to use, copy, modify, and/or distribute this software for any
// purpose with or without fee is hereby granted, provided that the above
// copyright notice and this permission notice appear in all copies.
//
// THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH
// REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY
// AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT,
// INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM
// LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR
// OTHER TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR
// PERFORMANCE OF THIS SOFTWARE.

use std::sync::atomic::AtomicBool;

pub static SIGNAL_HANDLED: AtomicBool = AtomicBool::new(false);

extern "C" fn handle_signal(s: std::ffi::c_int) {
    if s == libc::SIGINT || s == libc::SIGHUP || s == libc::SIGTERM {
        SIGNAL_HANDLED.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

pub fn register_signal_handlers() {
    unsafe {
        let mut sigaction = std::mem::MaybeUninit::<libc::sigaction>::zeroed();
        (*sigaction.as_mut_ptr()).sa_sigaction = handle_signal as *mut std::ffi::c_void as usize;
        libc::sigemptyset(&mut (*sigaction.as_mut_ptr()).sa_mask as *mut libc::sigset_t);
        libc::sigaction(libc::SIGINT, sigaction.as_ptr(), std::ptr::null_mut());
        libc::sigaction(libc::SIGHUP, sigaction.as_ptr(), std::ptr::null_mut());
        libc::sigaction(libc::SIGTERM, sigaction.as_ptr(), std::ptr::null_mut());
    }
}
