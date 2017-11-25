extern crate jit;
use jit::*;

#[test]
pub fn test_fib() {
	// make a new JIT context, with no metadata
	let mut ctx = Context::<()>::new();
	// make a function in this context with signature `fn(u64) -> u64`
    jit_func!(&mut ctx, func, fn(n: u64) -> u64 {
    	// compile constants
    	let two = func.insn_of(2u64);
    	let one = func.insn_of(1u64);
    	// the LibJIT type equivalent to a `u64`
    	let ty = get::<u64>();
    	// allocate temporary variables
    	let a = Val::new(&func, &ty);
    	let b = Val::new(&func, &ty);
    	let c = Val::new(&func, &ty);
    	// give temporary variables their initial values
    	func.insn_store(a, one);
    	func.insn_store(b, one);
    	// return 1 if n <= 2
    	func.build_if(func.insn_leq(n, two), || {
    		func.insn_return(one);
    	});
    	// loop n times and keep adding last two numbers
    	func.build_do_while(|| func.insn_gt(n, two),  || {
    		func.insn_store(c, func.insn_add(a, b));
    		func.insn_store(b, a);
    		func.insn_store(a, c);
    		func.insn_store(n, func.insn_sub(n, one));
    	});
    	// return the result of adding the very last two numbers
    	func.insn_return(c);
    	// optimize the function as much as possible
    	func.set_optimization_level(3);
    }, {
    	let fib = func;
    	assert_eq!(fib(1), 1);
    	assert_eq!(fib(2), 1);
    	assert_eq!(fib(3), 2);
    	assert_eq!(fib(4), 3);
    	assert_eq!(fib(5), 5);
    	assert_eq!(fib(6), 8);
    	assert_eq!(fib(7), 13);
    	assert_eq!(fib(8), 21);
    });
}