#[macro_use]
extern crate jit_macros;
extern crate jit;
use jit::*;

#[derive(Copy, Clone, Debug, PartialEq)]

#[repr(packed)]
#[derive(Compile)]
pub struct Position {
    pub x: f64,
    pub y: f64
}

#[test]
fn test_struct() {
    let mut ctx = Context::<()>::new();
    jit_func!(&mut ctx, func, fn(pos: *mut Position, mult: f64) -> () {
        let pos_ty = pos.get_type().get_ref().unwrap();
        println!("{:?}", pos_ty);
        println!("{:?}", pos_ty);
        println!("{}", pos_ty.fields().count());
        for field in pos_ty.fields() {
            println!("{:?}: {:?}: {:?}", field.get_name(), field.get_type(), field.get_offset());
        };
        let x = pos_ty.get_field("x").unwrap();
        let y = pos_ty.get_field("y").unwrap();
        func.insn_store_relative(pos, x.get_offset(), func.insn_mul(&pos["x"], mult));
        func.insn_store_relative(pos, y.get_offset(), func.insn_mul(&pos["y"], mult));
    }, {
        let mut pos = Position {
            x: 1.,
            y: 2.
        };
        func(&mut pos, 2.);
        assert_eq!(pos, Position {
            x: 2.,
            y: 4.
        });
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
