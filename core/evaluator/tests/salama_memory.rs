//! Fixed memory: strict code (`#[salama]`) never allocates while it runs. Counted with a global
//! allocator: a strict call doing a thousand times the work (loops, strict calls with list
//! arguments, list reads and writes) allocates no more than one doing it once — whatever the
//! host allocates around the call does not grow with the work done inside it.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct Counting;
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.realloc(ptr, layout, size) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

const SOURCE: &str = r#"
    thabiti N: Namba = 8

    #[salama]
    kazi wastani(sampuli: Orodha<Namba>) -> Namba {
        weka jumla = 0
        kwa i kutoka 0 hadi N {
            jumla += sampuli[i]
        }
        rejesha jumla / N
    }

    #[salama]
    kazi lainisha(sampuli: Orodha<Namba>) -> Namba {
        weka w = wastani(sampuli)
        kwa i kutoka 0 hadi N {
            sampuli[i] = (sampuli[i] + w) / 2
        }
        rejesha w
    }

    #[salama]
    kazi mara_moja(sampuli: Orodha<Namba>) -> Namba {
        weka x = 0
        kwa k kutoka 0 hadi 1 {
            x += lainisha(sampuli)
        }
        rejesha x
    }

    #[salama]
    kazi mara_elfu(sampuli: Orodha<Namba>) -> Namba {
        weka x = 0
        kwa k kutoka 0 hadi 1000 {
            x += lainisha(sampuli)
        }
        rejesha x
    }
"#;

#[test]
fn strict_code_allocates_nothing_while_it_runs() {
    if !asili_evaluator::nguvu::supported() {
        return;
    }
    let tokens = asili_lexer::tokenize(SOURCE).expect("tokenize");
    let module = asili_parser::parse_tokens(&tokens).expect("parse");
    let program = asili_evaluator::NativeProgram::build(&module).expect("build");
    let list = || {
        asili_evaluator::Value::list(
            (0..8)
                .map(|i| asili_evaluator::Value::Namba(i as f64))
                .collect(),
        )
    };
    let count = |name: &str| {
        let args = vec![list()];
        // Warm up once (frame pools), then count one call.
        program.call(name, vec![list()]).expect("warm-up");
        let before = ALLOCATIONS.load(Ordering::Relaxed);
        program.call(name, args).expect("call");
        ALLOCATIONS.load(Ordering::Relaxed) - before
    };
    let once = count("mara_moja");
    let thousand = count("mara_elfu");
    assert_eq!(
        thousand, once,
        "a thousand times the work allocated {thousand} times, once {once}"
    );
}
