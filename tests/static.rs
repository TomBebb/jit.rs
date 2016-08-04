#[no_link] #[macro_use]
extern crate jit_macros;
extern crate jit;
use jit::*;

const INITIAL: i64 = 0;
const FINAL: i64 = 100;

#[test]
fn test_length() {
    let mut var = INITIAL;
    let mut ctx = Context::<()>::new();
    jit_func!(&mut ctx, func, fn() -> () {
        let var_red = func.insn_of(&mut var);
        func.insn_store_relative(var_red, 0, func.insn_of(FINAL));
        func.insn_default_return();
    }, {
        func();
        assert_eq!(var, FINAL);
    });
}
