# Safety requirements of the Asili toolchain

The requirements the Asili language toolchain (`pata`, the `nguvu` backend, the runtime and the
device runtime `asili-kifaa`) meets for use in safety-related software. Each has an ID, the
requirement, why it exists, and how it is verified:

- **test** — automated tests carry a `Verifies: <ID>` comment; `tests/traceability.rs` fails
  when a requirement verified by test has no test, when a test names an unknown ID, or when
  [traceability.md](traceability.md) (generated from both) is out of date;
- **analysis** — a check that is not a test (a CI lint step, a review of the named code);
- **inspection** — review of the named code against the requirement.

IDs are stable: a requirement that is withdrawn keeps its ID, marked withdrawn. Hazards these
requirements control are in [risk-register.md](risk-register.md).

## Strict code

### REQ-STRICT-1: The strict subset is enforced at build time
A function marked `#[salama]` that uses anything outside the strict subset — a loop whose bounds
are not known at build time, `wakati`, `kwa … katika`, assigning a loop counter, recursion,
a call to a function that is not strict, an instruction that can allocate, a type other than
`Namba`, `Ukweli` and `Orodha<Namba>` — fails the build with an error naming the function and
the line.
Rationale: code whose time and memory are not bounded cannot be shown to meet a deadline.
Verification: test

### REQ-STRICT-2: The build reports a step and memory bound for every strict function
For each strict function, `pata jenga` prints an upper bound on the bytecode steps one call
executes (`hatua`) and on the frame memory it needs (`kumbukumbu`), callees included.
Rationale: the bounds are the input to the product's own timing and memory analysis.
Verification: test, analysis (`core/evaluator/src/salama.rs`, `Bounds`: the longest path through
loop bodies multiplied by their constant trip counts; not checked by counting executed steps,
which native code does not do)

### REQ-STRICT-3: Strict code allocates nothing while it runs
After its first call, a call to a strict function allocates no memory, whatever its inputs.
Rationale: allocation can fail or take unbounded time; a control step must do neither.
Verification: test

## Failure handling

### REQ-FAIL-1: An error leaving `kuu` enters the safe state
When an error leaves `kuu`, the program's `#[hali_salama]` function runs exactly once before
the program stops.
Rationale: outputs must be driven to a known safe position on any unrecoverable failure.
Verification: test

### REQ-FAIL-2: A watchdog that is not fed enters the safe state
After `mlinzi_anza(ms)`, if `mlinzi_lisha()` is not called within `ms` milliseconds, the safe
state runs once and the program exits with status 5.
Rationale: a stalled control loop must be detected and handled.
Verification: test

### REQ-FAIL-3: Passing the memory limit enters the safe state
After `kikomo_kumbukumbu(baiti)`, a program whose memory passes `baiti` bytes fails at the
runtime's next check, entering the safe state; the safe state itself runs without the limit.
Rationale: memory exhaustion must end in the safe state, not an undefined crash.
Verification: test

### REQ-FAIL-4: A fault inside the runtime enters the safe state
A panic inside the runtime (which shipped builds turn into an abort) first runs the safe state.
Rationale: an internal fault must still drive outputs to safe positions.
Verification: inspection (`core/evaluator/src/hali_salama.rs`, `register`: the panic hook)

### REQ-FAIL-5: The runtime has no panic paths of its own
Runtime modules (`host.rs`, `native.rs`, `eval/`, `value/`, `builtins/`, the allocator, the
safe-state and synchronization code) contain no `unwrap`, `expect`, `panic!`, `unreachable!`,
`todo!` or `unimplemented!` outside documented exceptions; impossible states are errors the
program sees, and poisoned locks are recovered.
Rationale: a crash is not a defined failure mode.
Verification: analysis (CI step "Runtime never panics": `cargo clippy -p asili-evaluator --lib`
with those lints denied)

### REQ-FAIL-6: The safe state is well formed
A `#[hali_salama]` function takes no parameters, and a program has at most one; otherwise the
build fails.
Rationale: the runtime calls it with nothing, and must know which one to call.
Verification: test

## Results

### REQ-NUM-1: Native code computes the language's reference results bit for bit
Every operator, conversion, comparison and method gives the language's reference result:
IEEE 754 binary64 arithmetic with its signed zeros, NaNs and infinities, remainders and floor
division of negative numbers, shifts, and integer arithmetic.
Rationale: a computation must not depend on how it was compiled.
Verification: test

### REQ-NUM-2: Integer lowering never changes a result
Native code computes in machine integers only where range analysis proves the values are exact
integers that fit; numbers beyond 2^53 stay floating point; division and remainder by constants
give exactly the reference result.
Rationale: faster integer code must not round differently.
Verification: test

### REQ-RUN-1: An index out of range is an error, never a memory access
Reading or writing a list element at an index outside the list is an error the program sees
(on a device, a failed call), never an access outside the list.
Verification: test

### REQ-RUN-2: Runaway recursion is an error
A chain of calls deeper than the limit fails with `undani mno` instead of exhausting the stack.
Verification: test

## The compiler

### REQ-COMP-1: Every function's intermediate code is verified
The intermediate code of every function is checked after lowering and after optimization (and
after each optimization pass in debug builds): every register and block exists, operand types
match, and no register is read before it is written on some path. A violation fails the build.
Rationale: an optimizer bug must be caught where it happens, not as wrong machine code.
Verification: test

### REQ-COMP-2: Optimized code computes what unoptimized code computes
On random programs, native code compiled with the optimizer gives the same results, bit for bit,
or the same error, as native code compiled without it.
Verification: test

### REQ-COMP-3: Builds are reproducible
The same source always builds byte-identical bytecode (`.asb`) and native images.
Rationale: the artifact shipped must be the artifact verified.
Verification: test

### REQ-COMP-4: Loaded bytecode is verified before it runs
Bytecode loaded from an `.asb` file whose instructions name a register, jump target, function
or constant that does not exist is rejected before any of it runs; the compiler's own output
passes the same check.
Rationale: native code reaches registers by address; a damaged file must not steer it.
Verification: test

### REQ-COMP-5: A native image runs only with the program it was built for
A native image built for another program, or damaged, is rejected when loaded.
Verification: test

## Targets

### REQ-TGT-1: The browser target computes what native code computes
Programs compiled to WebAssembly (the playground) print exactly what native code prints.
Verification: test (`driver/wasm/tests/agree.sh`, CI job "Native code as wasm")

### REQ-TGT-2: The Cortex-M target computes what native code computes
Strict functions built for Cortex-M (`pata jenga --lengo cortex-m`) give, on the device, the
same result bit for bit, or the same failure, as native code on the build machine — except `**`,
whose device implementation (`libm`) may differ in the last bit.
Verification: test

### REQ-TGT-3: The Cortex-M build rejects what a device cannot run
A strict function that needs anything a device does not have (the host's services, allocation,
lists held as integers, a callee writing a list it receives) fails the device build with an
error naming the function and the reason.
Verification: test

### REQ-TGT-4: Cortex-M instructions are encoded as the architecture defines
Every Thumb-2 and floating-point instruction the Cortex-M code generator emits is encoded
exactly as the LLVM assembler encodes it.
Verification: test

### REQ-TGT-5: Native code behaves the same on every supported host
The differential tests pass on x86-64 Linux, AArch64 Linux, Apple silicon macOS and x86-64
Windows.
Verification: analysis (CI job "Native code on …" runs `cargo test -p asili-evaluator` on each)
