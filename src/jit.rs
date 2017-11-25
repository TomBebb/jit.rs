/* Copyright (c) 2014, Peter Nelson
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions are met:
 *
 * 1. Redistributions of source code must retain the above copyright notice,
 *    this list of conditions and the following disclaimer.
 *
 * 2. Redistributions in binary form must reproduce the above copyright notice,
 *    this list of conditions and the following disclaimer in the documentation
 *    and/or other materials provided with the distribution.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
 * AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE
 * LIABLE FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR
 * CONSEQUENTIAL DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF
 * SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS
 * INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN
 * CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
 * ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE
 * POSSIBILITY OF SUCH DAMAGE.
 */
#![crate_name = "jit"]
#![allow(non_camel_case_types, non_upper_case_globals)]
#![deny(unused_attributes, dead_code, unused_parens, unknown_lints, unreachable_code, unused_allocation, unused_allocation, unused_must_use)]

//! This crate wraps LibJIT in an idiomatic style.
//! For example, here's a quick example which makes a multiply function using LibJIT:
//!
//! ```rust
//! #[macro_use]
//! extern crate jit;
//! use jit::*;
//! fn main() {
//!     // make a new context to make functions on
//!     let mut ctx = Context::<()>::new();
//!     jit_func!(&mut ctx, func, fn(x: isize, y: isize) -> isize {
//!         func.insn_return(x * y);
//!     }, {
//!         assert_eq!(func(4, 5), 20);
//!         assert_eq!(func(-2, -4), 8);
//!     });
//! }
//! ```
#[macro_use]
extern crate bitflags;
extern crate cbox;
extern crate libc;
extern crate libjit_sys as raw;
extern crate traitobject;
use raw::*;
use std::os::raw::c_void;
use std::mem;
pub use compile::Compile;
pub use context::{Context, ContextMember};
pub use elf::*;
pub use function::{flags, Abi, UncompiledFunction, Func, CompiledFunction};
pub use function::flags::CallFlags;
pub use label::Label;
pub use insn::{Block, Instruction, InstructionIter};
pub use types::TypeKind;
pub use types::{get, Type, Field, Fields, Params, CowType, StaticType, Ty, TaggedType};
pub use types::consts as typecs;
pub use value::Val;


extern fn free_data<T>(data: *mut c_void) {
    unsafe {
        let actual_data:Box<T> = mem::transmute(data);
        mem::drop(actual_data);
    }
}

/// Initialise the library and prepare for operations
#[inline]
pub fn init() -> () {
    unsafe {
        jit_init()
    }
}
/// Check if the JIT is using a fallback interpreter
#[inline]
pub fn uses_interpreter() -> bool {
    unsafe {
        jit_uses_interpreter() != 0
    }
}
/// Check if the JIT supports theads
#[inline]
pub fn supports_threads() -> bool {
    unsafe {
        jit_supports_threads() != 0
    }
}
/// Check if the JIT supports virtual memory
#[inline]
pub fn supports_virtual_memory() -> bool {
    unsafe {
        jit_supports_virtual_memory() != 0
    }
}
#[macro_use]
mod macros;
mod context;
mod compile;
mod elf;
mod function;
mod insn;
mod label;
mod types;
mod util;
mod value;


#[macro_export]
/// Construct a JIT struct with the fields given
macro_rules! jit_struct(
    ($($name:ident: $ty:ty),*) => ({
        let mut ty = Type::new_struct(&mut [
            $(&get::<$ty>()),*
        ]);
        ty.set_names(&[$(stringify!($name)),*]);
        ty
    });
    ($($ty:ty),+ ) => (
        Type::new_struct(&mut [
            $(&get::<$ty>()),+
        ])
    )
);

#[macro_export]
/// Construct a JIT union with the fields given
macro_rules! jit_union(
    ($($name:ident: $ty:ty),*) => ({
        let union = Type::new_union(&mut [
            $(&get::<$ty>()),*
        ]);
        union.set_names(&[$(stringify!($name)),*]);
        union
    });
    ($($ty:ty),+ ) => (
        Type::new_union(&mut [
            $(&get::<$ty>()),*
        ])
    )
);
#[macro_export]
/// Construct a JIT function signature with the arguments and return type given
macro_rules! jit_fn(
    (($($arg:ty),*) -> $ret:ty) => ({
        use std::default::Default;
        Type::new_signature(Default::default(), &get::<$ret>(), &mut [
            $(&get::<$arg>()),*
        ])
    });
);

#[macro_export]
macro_rules! jit(
    ($func:ident, return) => (
        $func.insn_default_return()
    );
    ($func:ident, return $($t:tt)+) => (
        $func.insn_return(jit!($func, $($t)+))
    );
    ($func:ident, $var:ident += $($t:tt)+) => (
        $func.insn_store($var, &$func.insn_add($var, jit!($func, $($t)+)));
    );
    ($func:ident, $var:ident -= $($t:tt)+) => (
        $func.insn_store($var, &$func.insn_sub($var, jit!($func, $($t)+)));
    );
    ($func:ident, $var:ident *= $($t:tt)+) => (
        $func.insn_store($var, &$func.insn_mul($var, jit!($func, $($t)+)));
    );
    ($func:ident, $var:ident /= $($t:tt)+) => (
        $func.insn_store($var, &$func.insn_div($var, jit!($func, $($t)+)));
    );
    ($func:ident, $($a:tt)+ + $($b:tt)+) => (
        $func.insn_add(jit!($func, $($a)+), jit!($func, $($b)+))
    );
    ($func:ident, $($a:tt)+ - $($b:tt)+) => (
        $func.insn_sub(jit!($func, $($a)+), jit!($func, $($b)+))
    );
    ($func:ident, $($a:tt)+ * $($b:tt)+) => (
        $func.insn_mul(jit!($func, $($a)+), jit!($func, $($b)+))
    );
    ($func:ident, $($a:tt)+ / $($b:tt)+) => (
        $func.insn_div(jit!($func, $($a)+), jit!($func, $($b)+))
    );
    ($func:ident, $($a:tt)+ % $($b:tt)+) => (
        $func.insn_rem(jit!($func, $($a)+), jit!($func, $($b)+))
    );
    ($func:ident, ($($t:tt)+).sqrt()) => (
        $func.insn_sqrt(&jit!($func, $($t)+))
    );
    ($func:ident, $var:ident = $($t:tt)+) => (
        $func.insn_store($var, jit!($func, $val));
    );
    ($func:ident, *$var:ident) => (
        $func.insn_load($var)
    );
    ($func:ident, call($call:expr,
        $($arg:expr),+
    )) => (
        $func.insn_call(None::<String>, $call, None, [$($arg),+].as_mut_slice())
    );
    ($func:ident, jump_table($value:expr,
        $($label:ident),+
    )) => (
    let ($($label),+) = {
        $(let $label:Label = Label::new($func);)+
        $func.insn_jump_table($value, [
            $($label),+
        ].as_mut_slice());
        ($($label),+)
    });
);
#[macro_export]
macro_rules! jit_func(
    ($ctx:expr, $name:ident, fn() -> $ret:ty {$($st:stmt;)+}, $value:expr) => ({
        use jit::*;
        let func = UncompiledFunction::new($ctx, &get::<fn() -> $ret>());
        {
            let $name = &func;
            $($st;)+
        };
        let func = UncompiledFunction::compile(func);
        let $name: extern fn(()) -> $ret = func.as_func(); 
        let $name: extern fn() -> $ret = unsafe { ::std::mem::transmute($name) };
        $value
    });
    ($ctx:expr, $name:ident, fn($($arg:ident:$ty:ty),+) -> $ret:ty {$($st:stmt;)+}, $value:expr) => ({
        use jit::*;
        let func = UncompiledFunction::new($ctx, &get::<fn($($ty),+) -> $ret>());
        {
            let $name = &func;
            let mut i = 0;
            $(let $arg = {
                i += 1;
                &$name[i - 1]
            };)*
            $($st;)+
        };
        let func = UncompiledFunction::compile(func);
        let $name: extern fn(($($ty),+)) -> $ret = func.as_func();
        let $name: extern fn($($ty),+) -> $ret = unsafe { ::std::mem::transmute($name) };
        $value
    });
);
