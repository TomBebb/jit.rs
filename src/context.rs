use raw::*;
use function::Func;
use util::{oom, from_ptr_opt};
use std::default::Default;
use std::marker::PhantomData;
use std::{mem, ptr};
use std::ops::{Index, IndexMut};
use cbox::{CBox, DisposeRef};
/// Holds all of the functions you have built and compiled. There can be
/// multiple, but normally there is only one.
///
/// The type parameter `T` represents the type of the tagged data on the
/// context, which can be indexed to get this data. If you do not want
/// to tag the context with data, make sure to instantiate it with a
/// unit (`()`) for `T`, like so:
///
/// ```rust
/// use jit::Context;
/// let ctx = Context::<()>::new();
/// ```
/// However, if you do want to set tagged data on it, simply put the type
/// of the data as the `T` parameter when you instantiate it, like so:
///
/// ```rust
/// use jit::Context;
/// let mut ctx = Context::<usize>::new();
/// ctx[0] = 42;
/// ctx[1] = 21;
/// assert_eq!(ctx[0], 42);
/// assert_eq!(ctx[1], 21);
/// ```
pub struct Context<T = ()> {
    _marker: PhantomData<T>
}
impl<'a, T> Into<jit_context_t> for &'a Context<T> {
    fn into(self) -> jit_context_t {
        unsafe { mem::transmute(self) }
    }
}
impl<'a, T> Into<jit_context_t> for &'a mut Context<T> {
    fn into(self) -> jit_context_t {
        unsafe { mem::transmute(self) }
    }
}
impl<'a, T> From<jit_context_t> for &'a Context<T> {
    fn from(ty: jit_context_t) -> &'a Context<T> {
        unsafe { mem::transmute(ty) }
    }
}
impl<T> DisposeRef for Context<T> {
    type RefTo = Struct__jit_context;
    unsafe fn dispose(c: jit_context_t) {
        jit_context_destroy(c);
    }
}

impl<T> Index<i32> for Context<T> {
    type Output = T;
    fn index(&self, index: i32) -> &T {
        unsafe {
            let meta = jit_context_get_meta(self.into(), index);
            if meta.is_null() {
                panic!("No such index {} on Context", index)
            }
            mem::transmute(meta)
        }
    }
}
impl<T> IndexMut<i32> for Context<T> where T: Default {
    fn index_mut(&mut self, index: i32) -> &mut T {
        unsafe {
            let meta = jit_context_get_meta(self.into(), index);
            if meta.is_null() {
                let boxed = Box::new(T::default());
                if jit_context_set_meta(self.into(), index, mem::transmute(boxed), Some(::free_data::<T>)) == 0 {
                    oom()
                } else {
                    mem::transmute(jit_context_get_meta(self.into(), index))
                }
            } else {
                mem::transmute(meta)
            }
        }
    }
}
impl<T> Context<T> {
    #[inline(always)]
    /// Create a new JIT Context
    pub fn new() -> CBox<Context<T>> {
        unsafe {
            CBox::new(jit_context_create())
        }
    }
    /// Iterate through the functions contained inside this context
    pub fn functions(&self) -> Functions {
        Functions {
            context: self.into(),
            last: ptr::null_mut(),
            lifetime: PhantomData,
        }
    }
}
impl<T> Drop for Context<T> {
    #[inline(always)]
    fn drop(&mut self) {
        unsafe {
            jit_context_destroy(self.into());
        }
    }
}

pub struct Functions<'a> {
    context: jit_context_t,
    last: jit_function_t,
    lifetime: PhantomData<&'a ()>
}
impl<'a> Iterator for Functions<'a> {
    type Item = &'a Func;
    fn next(&mut self) -> Option<&'a Func> {
        unsafe {
            self.last = jit_function_next(self.context, self.last);
            from_ptr_opt(self.last)
        }
    }
}

/// An interface to be implemented by all structs that are members of a `Context`.
pub trait ContextMember {
    /// Get the context that this is a member of.
    fn context(&self) -> &Context;
}