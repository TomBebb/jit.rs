use raw::*;
use context::{Context, ContextMember};
use function::{UncompiledFunction, FunctionMember};
use types::Ty;
use label::Label;
use util::{from_ptr, from_ptr_opt};
use value::Val;
use std::{ffi, fmt, mem, str};
use std::marker::PhantomData;

/// Represents a single LibJIT instruction
pub struct Instruction(PhantomData<[()]>);
native_ref!(&Instruction = jit_insn_t);

impl ContextMember for Instruction {
	fn get_context(&self) -> &Context {
		unsafe {
			from_ptr(jit_block_get_context(self.into()))
		}
	}
}
impl FunctionMember for Instruction {
	fn get_function(&self) -> &UncompiledFunction {
		unsafe {
			from_ptr(jit_block_get_function(self.into()))
		}
	}
}
impl Instruction {
	/// Get the opcode of the instruction
	pub fn get_opcode(&self) -> i32 {
		unsafe {
			jit_insn_get_opcode(self.into())
		}
	}
	/// Get the destination value
	pub fn get_dest(&self) -> Option<&Val> {
		unsafe {
			from_ptr_opt(jit_insn_get_dest(self.into()))
		}
	}
	/// Get if the destination value is a value
	pub fn dest_is_value(&self) -> bool {
		unsafe {
			jit_insn_dest_is_value(self.into()) != 0
		}
	}
	/// Get the left value
	pub fn get_value1(&self) -> Option<&Val> {
		unsafe {
			from_ptr_opt(jit_insn_get_value1(self.into()))
		}
	}
	/// Get the right value
	pub fn get_value2(&self) -> Option<&Val> {
		unsafe {
			from_ptr_opt(jit_insn_get_value2(self.into()))
		}
	}
	/// Get the function containing this value
	pub fn get_function(&self) -> Option<&UncompiledFunction> {
		unsafe {
			from_ptr_opt(jit_insn_get_function(self.into()))
		}
	}
	/// Get the signature of this value
	pub fn get_signature(&self) -> Option<&Ty> {
		unsafe {
			from_ptr_opt(jit_insn_get_signature(self.into()))
		}
	}
	/// Get the name of the instruction
	pub fn get_name(&self) -> &str {
		unsafe {
			let c_name = jit_insn_get_name(self.into());
            let c_name = ffi::CStr::from_ptr(c_name);
			str::from_utf8(c_name.to_bytes()).unwrap()
		}
	}
}
impl fmt::Debug for Instruction {
	fn fmt(&self, fmt:&mut fmt::Formatter) -> fmt::Result {
		let v1 = self.get_value1().map(|v| format!("{:?}", v)).unwrap_or_else(String::new);
		let v2 = self.get_value2().map(|v| format!("{:?}", v)).unwrap_or_else(String::new);
		write!(fmt, "{}({}, {})", self.get_name(), v1, v2)
	}
}

pub struct InstructionIter<'a> {
	_iter: jit_insn_iter_t,
	marker: PhantomData<&'a ()>,
}
impl<'a> Iterator for InstructionIter<'a> {
	type Item = &'a Instruction;
	fn next(&mut self) -> Option<&'a Instruction> {
		unsafe {
			let ptr = jit_insn_iter_next(&mut self._iter);
            from_ptr_opt(ptr as jit_insn_t)
		}
	}
}

/// Represents a single LibJIT block
pub struct Block(PhantomData<[()]>);
native_ref!(&Block = jit_block_t);
impl ContextMember for Block {
	fn get_context(&self) -> &Context {
		self.get_function().get_context()
	}
}
impl FunctionMember for Block {
	fn get_function(&self) -> &UncompiledFunction {
		unsafe {
			from_ptr(jit_block_get_function(self.into()))
		}
	}
}
impl Block {
	/// Get the block corresponding to a particular label
	pub fn from_label<'a>(func: &'a UncompiledFunction, label: Label<'a>) -> Option<&'a Block> {
		unsafe {
			from_ptr_opt(jit_block_from_label(func.into(), *label))
		}
	}
	/// Get the function containing this block
	pub fn get_function(&self) -> &UncompiledFunction {
		unsafe {
			from_ptr(jit_block_get_function(self.into()))
		}
	}
	/// Check if the block is reachable
	pub fn is_reachable(&self) -> bool {
		unsafe {
			jit_block_is_reachable(self.into()) != 0
		}
	}
	/// Check if the block ends in dead code
	pub fn ends_in_dead(&self) -> bool {
		unsafe {
			jit_block_ends_in_dead(self.into()) != 0
		}
	}
	/// Iterate through the instructions
	pub fn iter(&self) -> InstructionIter {
		unsafe {
			let mut iter = mem::zeroed();
			jit_insn_iter_init(&mut iter, self.into());
			debug_assert!(iter.block == self.into());
			debug_assert!(iter.posn == 0);
			InstructionIter {
				_iter: iter,
				marker: PhantomData
			}
		}
	}
}
