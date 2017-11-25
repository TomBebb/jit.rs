#[macro_use]
extern crate jit;
use jit::*;
use std::default::Default;
macro_rules! test_compile(
    ($ty:ty, $test_name:ident, $id:ident, $kind:ident) => (
        #[test]
        fn $test_name() {
            let default_value:$ty = Default::default();
            let ty = get::<$ty>();
            assert!(ty.get_kind().contains(TypeKind::$kind));
            assert_eq!(typecs::$id(), &*ty);
            let mut ctx = Context::<()>::new();
            jit_func!(&mut ctx, gen, fn() -> $ty {
                let val = gen.insn_of(default_value);
                gen.insn_return(val);
            }, assert_eq!(gen(), default_value));
        }
    );
);
test_compile!(f64, test_compile_f64, get_float64, Float64);
test_compile!(f32, test_compile_f32, get_float32, Float32);
test_compile!(isize, test_compile_isize, get_nint, NInt);
test_compile!(usize, test_compile_usize, get_nuint, NUInt);
test_compile!(i64, test_compile_i64, get_long, Long);
test_compile!(u64, test_compile_u64, get_ulong,  ULong);
test_compile!(i32, test_compile_i32, get_int, Int);
test_compile!(u32, test_compile_u32, get_uint, UInt);
test_compile!(i16, test_compile_i16, get_short, Short);
test_compile!(u16, test_compile_u16, get_ushort, UShort);
test_compile!(i8, test_compile_i8, get_sbyte, SByte);
test_compile!(u8, test_compile_u8, get_ubyte,  UByte);