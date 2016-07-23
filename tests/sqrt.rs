#[no_link] #[macro_use]
extern crate jit_macros;
extern crate jit;
use jit::*;

#[test]
fn test_sqrt() {
    let mut ctx = Context::<()>::new();
    assert_eq!(ctx.functions().count(), 0);
    jit_func!(&mut ctx, func, fn(num: usize) -> usize {
        let num = func.insn_convert(num, &get::<f64>(), false);
        let val = func.insn_sqrt(num);
        func.insn_return(val);
    }, {
        assert_eq!(func(64), 8);
        assert_eq!(func(16), 4);
        assert_eq!(func(9), 3);
        assert_eq!(func(4), 2);
        assert_eq!(func(1), 1);
    });
}
