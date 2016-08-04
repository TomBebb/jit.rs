#[no_link] #[macro_use]
extern crate jit_macros;
extern crate jit;
use jit::*;
use std::mem;

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(packed)]
pub struct Position(f64, f64);
impl<'a> Compile<'a> for Position {
    fn compile(self, func:&'a UncompiledFunction) -> &'a Val {
        let val = Val::new(func, &Self::get_type());
        func.insn_store_relative(val, 0, func.insn_of(self.0));
        func.insn_store_relative(val, 8, func.insn_of(self.1));
        val
    }
    fn get_type() -> CowType<'a> {
        let f64_t = get::<f64>();
        Type::new_struct(&mut [&f64_t, &f64_t]).into()
    }
}


#[test]
fn test_struct() {
    let mut ctx = Context::<()>::new();
    jit_func!(&mut ctx, func, fn(pos: *const Position, mult: f64) -> () {
        let f64_t = get::<f64>();
        let mut x = func.insn_load_relative(pos, 0, &f64_t);
        let mut y = func.insn_load_relative(pos, f64_t.get_size(), &f64_t);
        x = func.insn_mul(x, mult);
        y = func.insn_mul(y, mult);
        func.insn_store_relative(pos, 0, x);
        func.insn_store_relative(pos, f64_t.get_size(), y);
    }, {
        let pos = Position(1., 2.);
        func(&pos, 2.);
        assert_eq!(pos, Position(2., 4.));
    });
}
