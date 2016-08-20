#[no_link] #[macro_use]
extern crate jit_macros;
extern crate jit;
use jit::*;
use std::cell::Cell;

#[test]
fn test_closure() {
    let mut ctx = Context::<()>::new();
    let mut num: u32 = 1;
    let mut add_num = |n: u32| {
        num += n
    };
    jit_func!(&mut ctx, func, fn(n: u32) -> () {
        func.insn_call_rust_mut(Some("add_num"), &mut add_num, &[n], flags::NO_THROW);
        func.insn_default_return();
    }, {
        assert_eq!(num, 1);
        func(1);
        assert_eq!(num, 2);
    });
}

#[test]
fn test_closure_cell() {
    let mut ctx = Context::<()>::new();
    let num = Cell::new(1u32);
    let set_num = |n: u32| {
        num.set(n);
    };
    jit_func!(&mut ctx, func, fn(n: u32) -> () {
        func.insn_call_rust(Some("set_num"), &set_num, &[n], flags::NO_THROW);
        func.insn_default_return();
    }, {
        assert_eq!(num, 1);
        func(2);
        assert_eq!(num, 2);
    });
}