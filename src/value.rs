use raw::*;
use function::UncompiledFunction;
use types::*;
use compile::Compile;
use context::{Context, ContextMember};
use std::marker::PhantomData;
use std::{fmt, mem, ptr};
use std::ops::*;
use util;
/// Vals form the backbone of the storage system in `LibJIT`
///
/// Every value in the system, be it a constant, a local variable, or a
/// temporary result, is represented by an object of type `Val`. The JIT then
/// allocates registers or memory locations to the values as appropriate. This is
/// why `Val` is always behind a reference
pub struct Val(PhantomData<[()]>);
native_ref!(&Val = jit_value_t);
impl fmt::Debug for Val {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let func = self.get_function();
        write!(fmt, "{}", try!(util::dump(|fd| unsafe {
            jit_dump_value(mem::transmute(fd), func.into(), self.into(), ptr::null());
        })))
    }
}
impl ContextMember for Val {
    /// Get the context this value is contained in
    fn get_context(&self) -> &Context<()> {
        unsafe { jit_value_get_context(self.into()).into() }
    }
}
impl Val {
    #[inline(always)]
    /// Create a new value in the context of a function's current block.
    ///
    /// The value initially starts off as a block-specific temporary. It will be
    /// converted into a function-wide local variable if it is ever referenced
    /// from a different block.
    ///
    /// Basically, use this for allocating values you want to be able to 
    /// access and change throughout the function.
    pub fn new<'a>(func:&'a UncompiledFunction, value_type:&Ty) -> &'a Val {
        unsafe {
            jit_value_create(func.into(), value_type.into()).into()
        }
    }
    /// Create a new instance of the struct `ty` in `func` with the fields `fields`.
    pub fn new_struct<'a>(func: &'a UncompiledFunction, ty: &Ty, fields: &[&'a Val]) -> &'a Val {
        let value = Val::new(func, ty);
        for (index, field) in ty.fields().enumerate() {
            func.insn_store_relative(value, field.get_offset(), fields[index])
        }
        value
    }
    /// Get the type of the value
    pub fn get_type(&self) -> &Ty {
        unsafe {
            jit_value_get_type(self.into()).into()
        }
    }
    /// Get the function which made this value
    pub fn get_function(&self) -> &UncompiledFunction {
        unsafe {
            jit_value_get_function(self.into()).into()
        }
    }
    /// Determine if a value is temporary.  i.e. its scope extends over a single
    /// block within its function.
    #[inline]
    pub fn is_temp(&self) -> bool {
        unsafe {
            jit_value_is_temporary(self.into()) != 0
        }
    }
    /// Determine if a value is addressable.
    #[inline]
    pub fn is_addressable(&self) -> bool {
        unsafe {
            jit_value_is_addressable(self.into()) != 0
        }
    }
    /// Set a flag on a value to indicate that it is addressable.
    /// This should be used when you want to take the address of a value (e.g.
    /// `&variable` in Rust/C).  The value is guaranteed to not be stored in a
    /// register across a function call.
    #[inline]
    pub fn set_addressable(&self) -> () {
        unsafe {
            jit_value_set_addressable(self.into())
        }
    }
}
impl Index<usize> for Val {
    type Output = Val;
    fn index(&self, index: usize) -> &Val {
        let func = self.get_function();
        let mut ty = self.get_type();
        while let Some(elem) = ty.get_ref() {
            ty = elem;
        }
        if !ty.is_struct() {
            panic!("{:?} cannot be indexed", ty);
        } else if let Some(field) = ty.fields().nth(index) {
            func.insn_load_relative(self, field.get_offset(), field.get_type())
        } else {
            panic!("unknown index {} on {:?}", index, ty)
        }
    }
}
impl<'a> Index<&'a str> for Val {
    type Output = Val;
    fn index(&self, index: &'a str) -> &Val {
        let func = self.get_function();
        let mut ty = self.get_type();
        while let Some(elem) = ty.get_ref() {
            ty = elem;
        }
        if !ty.is_struct() {
            panic!("{:?} cannot be indexed", ty);
        } else if let Some(field) = ty.get_field(index) {
            func.insn_load_relative(self, field.get_offset(), field.get_type())
        } else {
            panic!("unknown field {:?} on {:?}", index, ty)
        }
    }
}
macro_rules! bin_op {
    ($trait_ty:ident, $trait_func:ident, $assign_ty:ident, $assign_func:ident, $func:ident) => (
        impl<'a> $trait_ty<&'a Val> for &'a Val {
            type Output = &'a Val;
            fn $trait_func(self, other: &'a Val) -> &'a Val {
                self.get_function().$func(self, other)
            }
        }
        impl<'a, T> $trait_ty<T> for &'a Val where T: Compile<'a> {
            type Output = &'a Val;
            fn $trait_func(self, other: T) -> &'a Val {
                let func = self.get_function();
                func.$func(self, func.insn_of(other))
            }
        }
        impl<'a> $assign_ty<&'a Val> for &'a Val {
            fn $assign_func(&mut self, other: &'a Val) {
                let func = self.get_function();
                func.insn_store(*self, func.$func(self, other));
            }
        }
        impl<'a, T> $assign_ty<T> for &'a Val where T: Compile<'a> {
            fn $assign_func(&mut self, other: T) {
                let func = self.get_function();
                func.insn_store(*self, func.$func(self, func.insn_of(other)));
            }
        }
    )
}
macro_rules! un_op {
    ($trait_ty:ident, $trait_func:ident, $func:ident) => (
        impl<'a> $trait_ty for &'a Val {
            type Output = &'a Val;
            fn $trait_func(self) -> &'a Val {
                self.get_function().$func(self)
            }
        }
    )
}
bin_op!{Add, add, AddAssign, add_assign, insn_add}
bin_op!{BitAnd, bitand, BitAndAssign, bitand_assign, insn_and}
bin_op!{BitOr, bitor, BitOrAssign, bitor_assign, insn_or}
bin_op!{BitXor, bitxor, BitXorAssign, bitxor_assign, insn_xor}
bin_op!{Div, div, DivAssign, div_assign, insn_div}
bin_op!{Mul, mul, MulAssign, mul_assign, insn_mul}
bin_op!{Rem, rem, RemAssign, rem_assign, insn_rem}
bin_op!{Shl, shl, ShlAssign, shl_assign, insn_shl}
bin_op!{Shr, shr, ShrAssign, shr_assign, insn_shr}
bin_op!{Sub, sub, SubAssign, sub_assign, insn_sub}
un_op!{Neg, neg, insn_neg}
un_op!{Not, not, insn_not}
