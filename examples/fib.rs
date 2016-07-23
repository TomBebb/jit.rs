#[no_link] #[macro_use]
extern crate jit_macros;
extern crate jit;
use jit::*;

pub fn main() {
	let mut ctx = Context::<()>::new();
    jit_func!(&mut ctx, func, fn(n: u32) -> u32 {
    	let two = func.insn_of(2u32);
    	let one = func.insn_of(1u32);
    	let u32_t = get::<u32>();
    	func.build_if_else(func.insn_leq(n, two), || {
    		func.insn_return(one);
    	}, || {
    		let a = Val::new(&func, &u32_t);
    		let b = Val::new(&func, &u32_t);
    		let c = Val::new(&func, &u32_t);
    		let i = Val::new(&func, &u32_t);
    		func.insn_store(a, one);
    		func.insn_store(b, one);
    		func.insn_store(i, func.insn_of(0u32));
    		let right = func.insn_sub(n, two);
    		func.build_while(|| {
    			let left = func.insn_load(i);
    			func.insn_lt(left, right)
    		}, || {
    			func.insn_store(c, func.insn_add(func.insn_load(a), func.insn_load(b)));
    			func.insn_store(b, func.insn_load(a));
    			func.insn_store(a, func.insn_load(c));
    			func.insn_store(i, func.insn_add(func.insn_load(i), one));
    		});
    		func.insn_return(func.insn_load(a));
    	});
    	func.set_optimization_level(3);
    }, {
    	assert_eq!(fib(0), 1);
    	assert_eq!(fib(1), 1);
    	assert_eq!(fib(2), 2);
    	assert_eq!(fib(3), 3);
    	assert_eq!(fib(4), 5);
    	assert_eq!(fib(5), 8);
    	assert_eq!(fib(6), 13);
    	assert_eq!(fib(6), 21);
    });
}