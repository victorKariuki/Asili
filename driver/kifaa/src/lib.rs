//! `asili-kifaa`: the runtime a device links beside the object `pata jenga --lengo cortex-m`
//! writes. Strict code calls these functions by name (`asili_kifaa_*`) for what it does not do
//! inline: reading the caller's lists, `%` and `**` on numbers, and the conversions of the
//! bitwise operators. No allocator, no threads, no files: build it for the device with
//! `cargo build --release -p asili-kifaa --target thumbv7em-none-eabihf` and link
//! `libasili_kifaa.a` (it also provides the EABI 64-bit division helpers the code calls).
//!
//! Each function computes exactly what the host's runtime does (`core/evaluator/src/native.rs`);
//! `pow` comes from `libm`, whose results can differ from the host's C library in the last bit.

#![cfg_attr(not(test), no_std)]

/// A list as the caller passes it: its `double` elements and their count (`asili_orodha` in
/// the generated header).
#[repr(C)]
pub struct Orodha {
    pub data: *mut f64,
    pub len: u32,
}

/// `Kind::F64.code()`: the only representation a device list has.
const F64_CODE: u32 = 7;

/// Status of a call that cannot be made on a device (`native::STATUS_FAIL << 32`).
const FAIL: u64 = 2 << 32;

/// Data pointer of list register `reg` of `frame` (an array of list references), or null when
/// the code wants the list held another way.
///
/// # Safety
/// `frame` points to at least `reg + 1` list references, as the generated code passes it.
#[no_mangle]
pub unsafe extern "C" fn asili_kifaa_list_ptr(frame: *const Orodha, reg: u32, want: u32) -> u64 {
    if want != F64_CODE {
        return 0;
    }
    (*frame.add(reg as usize)).data as usize as u64
}

/// Length of list register `reg` of `frame`.
///
/// # Safety
/// As [`asili_kifaa_list_ptr`].
#[no_mangle]
pub unsafe extern "C" fn asili_kifaa_list_len(frame: *const Orodha, reg: u32) -> i64 {
    (*frame.add(reg as usize)).len as i64
}

#[no_mangle]
pub extern "C" fn asili_kifaa_fmod(a: f64, b: f64) -> f64 {
    libm::fmod(a, b)
}

#[no_mangle]
pub extern "C" fn asili_kifaa_pow(a: f64, b: f64) -> f64 {
    libm::pow(a, b)
}

/// Saturating `f64 as i64` (NaN → 0).
#[no_mangle]
pub extern "C" fn asili_kifaa_float_to_int_sat(a: f64) -> i64 {
    a as i64
}

/// The language's shift amount: `f64 as i32`, anything outside `0..=63` → 0.
#[no_mangle]
pub extern "C" fn asili_kifaa_shift_amount(a: f64) -> i64 {
    let shift = a as i32;
    if (0..=63).contains(&shift) {
        shift as i64
    } else {
        0
    }
}

/// An instruction native code hands to the host. On a device that only happens on an error
/// path (an index out of range): the call fails (`native::STATUS_FAIL`).
#[no_mangle]
pub extern "C" fn asili_kifaa_exec(
    _host: *mut u8,
    _frame: *mut u8,
    _function: u32,
    _pc: u32,
) -> u64 {
    2
}

/// A direct call too deep for the stack: a device has no host call path, so the call fails.
#[no_mangle]
pub extern "C" fn asili_kifaa_call_host(_host: *mut u8, _function: u32, _nums: *mut f64) -> u64 {
    FAIL
}

/// The call depth reached its limit.
#[no_mangle]
pub extern "C" fn asili_kifaa_depth_error(_host: *mut u8) -> u64 {
    FAIL
}

#[cfg(all(not(test), target_os = "none"))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

// On a hosted target (building or testing the crate on the development machine) the standard
// library supplies the panic handler.
#[cfg(all(not(test), not(target_os = "none")))]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_host_runtime() {
        assert_eq!(asili_kifaa_shift_amount(70.0), 0);
        assert_eq!(asili_kifaa_shift_amount(5.9), 5);
        assert_eq!(asili_kifaa_float_to_int_sat(f64::NAN), 0);
        assert_eq!(asili_kifaa_fmod(-7.0, 3.0), -1.0);
        let mut data = [1.0, 2.0];
        let lists = [Orodha {
            data: data.as_mut_ptr(),
            len: 2,
        }];
        unsafe {
            assert_eq!(asili_kifaa_list_len(lists.as_ptr(), 0), 2);
            assert_eq!(asili_kifaa_list_ptr(lists.as_ptr(), 0, 0), 0);
            assert_eq!(
                asili_kifaa_list_ptr(lists.as_ptr(), 0, F64_CODE),
                data.as_mut_ptr() as usize as u64
            );
        }
    }
}
