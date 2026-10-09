# Build strict code for Cortex-M

`pata jenga --lengo cortex-m` compiles a program's strict functions (`#[salama]`, see
`docs/language/07-mfumo-wa-aina.md`, "Strict code") to Thumb-2 machine code for ARMv7E-M with
the double-precision `fpv5-d16` floating-point unit (Cortex-M7, and Cortex-M4F/M33-class parts
with a double-precision unit), using Asili's own backend — no external compiler is involved in
producing the code. The result is an object file and a C header that a firmware project links
like any other library.

## Build

```
$ cd examples/kidhibiti
$ pata jenga --lengo cortex-m
salama: kazi 'dhibiti' — hatua ≤ 69, kumbukumbu ≤ baiti 200
salama: kazi 'wastani' — hatua ≤ 49, kumbukumbu ≤ baiti 104
msimbo asilia: ./kilele/kidhibiti.nguvu
kifaa (Cortex-M): ./kilele/kidhibiti-cortex-m.o na ./kilele/kidhibiti.h — kazi: wastani, dhibiti
imejengwa: ./kilele/kidhibiti.asb
```

`[jenga] lengo = "cortex-m"` in `pata.toml` does the same without the flag, and
`#[sharti(lengo = "cortex-m")]` selects items for this target as for any other.

- `kilele/<jina>-cortex-m.o` — an ELF32 relocatable object (ARM, EABI 5, hard-float ABI) with
  one global function per strict function, `asili_<kazi>`.
- `kilele/<jina>.h` — their C declarations.

The device runtime is a small `no_std` Rust library with no allocator, threads or files. Build
it once per toolchain:

```
$ rustup target add thumbv7em-none-eabihf
$ cargo build --release -p asili-kifaa --target thumbv7em-none-eabihf
# → target/thumbv7em-none-eabihf/release/libasili_kifaa.a
```

It provides the few functions the generated code calls by name (`asili_kifaa_*`: reading the
caller's lists, `%` and `**` on numbers, the conversions behind the bitwise operators) and the
EABI 64-bit division helpers (`__aeabi_ldivmod`, `__aeabi_uldivmod`). A firmware that already
links compiler-rt or libgcc gets the division helpers from there as well.

## Call from C

```c
#include "kidhibiti.h"

static double sampuli[8];

void control_tick(double lengo, double *marekebisho) {
    asili_dhibiti_hoja hoja = { { sampuli, 8 }, lengo };
    if (asili_dhibiti(&hoja, marekebisho) != 0) {
        enter_safe_state();   /* the call failed: the output was not written */
    }
}
```

Link `kidhibiti-cortex-m.o` and `libasili_kifaa.a` into the firmware (for example
`arm-none-eabi-gcc -mcpu=cortex-m7 -mfpu=fpv5-d16 -mfloat-abi=hard ... kidhibiti-cortex-m.o
libasili_kifaa.a`).

The calling convention, for every export:

- `int32_t asili_<kazi>(const asili_<kazi>_hoja *hoja, double *matokeo)` (a function with no
  parameters takes `const void *hoja`; pass `NULL`).
- `hoja` holds the arguments in order, eight bytes each: a `Namba` or `Ukweli` as a `double`
  (`Ukweli` as 0 or 1), an `Orodha<Namba>` as `asili_orodha { double *data; uint32_t len; }`
  pointing at the caller's array.
- The result is stored at `matokeo` (a `double`; `Ukweli` as 0 or 1; nothing for `Tupu`).
- The return value is 0 on success, non-zero when the call failed (an index out of range, for
  instance); `matokeo` is then not written, and the firmware should enter its safe state.
- A function that assigns to an element of a list parameter writes the caller's array: the list
  is passed by reference, not copied (copying would need memory the device does not allocate).
- Each call uses only the stack: the export builds the function's register frame on the stack
  (the `kumbukumbu` bound `pata jenga` prints is its frame memory) and allocates nothing.

## What runs on a device

Strict code already excludes allocation, unbounded loops, recursion and calls to non-strict
code. On a device, additionally:

- Exported functions take only `Namba`, `Ukweli` and `Orodha<Namba>` parameters.
- A strict function may call strict functions taking numbers directly; one taking a list is
  inlined into its caller, so it must not write to the list it receives.
- Lists are the caller's `double` arrays; code that would keep a list as narrower integers is
  rejected.

Anything else fails the build, naming the function and the reason
(`kazi salama 'f' haiwezi kujengwa kwa Cortex-M: ...`).

## Results are the same as on the build machine

Integers keep their full 64-bit width (register pairs on the 32-bit core), floating point is
IEEE double, and every operator has the same semantics as native code elsewhere. `tests/kifaa.rs`
builds the `kidhibiti` example, hand-written cases (big integers, bitwise operators, shifts,
`//` and `%`, calls, NaN, infinities, −0) and random strict programs for the device, runs them
under `qemu-arm -cpu cortex-m7`, and requires every result — or failure — to equal native code's
on the build machine bit for bit (CI runs 400 random programs). The one exception is `**`: the
device's `pow` comes from `libm` and can differ from the build machine's C library in the last
bit.

## Not yet

- Bare-metal run-time support (vector table, start-up code) and RTOS integration (Zephyr,
  FreeRTOS tasks and timing) are the firmware's: the object is a library, not a firmware image.
- Cycle-accurate worst-case timing for a specific core: `hatua` bounds the bytecode steps of a
  call; measuring it in cycles on the target part is part of the device's own verification.
