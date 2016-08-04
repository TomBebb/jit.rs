#[no_link] #[macro_use]
extern crate jit_macros;
extern crate jit;
use jit::*;

#[test]
fn test_length() {
    let mut ctx = Context::<()>::new();
    assert_eq!(ctx.functions().count(), 0);
    jit_func!(&mut ctx, func, fn(x: f64, y: f64) -> f64 {
        let x_sq = func.insn_mul(x, x);
        let y_sq = func.insn_mul(y, y);
        func.insn_sqrt(func.insn_add(x_sq, y_sq));
    }, {
        assert_eq!(func(3., 4.), 5.0);
        assert_eq!(func(4., 3.), 5.0);
    });
}
