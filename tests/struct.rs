#[no_link] #[macro_use]
extern crate jit_macros;
extern crate jit;
use jit::*;

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(packed)]
pub struct Position(f64, f64);
impl<'a> Compile<'a> for Position {
    fn compile(self, func:&'a UncompiledFunction) -> &'a Val {
        Val::new_struct(func, &Self::get_type(), &[func.insn_of(self.0), func.insn_of(self.1)])
    }
    fn get_type() -> CowType<'a> {
        let f64_t = get::<f64>();
        let mut ty = Type::new_struct(&mut [&f64_t, &f64_t]);
        ty.set_names(&["x", "y"]);
        ty.into()
    }
}


#[test]
fn test_struct() {
    let mut ctx = Context::<()>::new();
    jit_func!(&mut ctx, func, fn(pos: *mut Position, mult: f64) -> () {
        let mut x = &pos["x"];
        let mut y = &pos["y"];
        x *= mult;
        y *= mult;
    }, {
        let mut pos = Position(1., 2.);
        func(&mut pos, 2.);
        assert_eq!(pos, Position(2., 4.));
    });
}

#[test]
fn test_tuple() {
    let mut ctx = Context::<()>::new();
    jit_func!(&mut ctx, func, fn(thing: *const (i32, i32)) -> i32 {
        let x = &thing[0];
        let y = &thing[1];
        func.insn_return(x * y);
    }, {
        assert_eq!(func(&(2, 5)), 10);
        assert_eq!(func(&(1, -32)), -32);
    });
}
