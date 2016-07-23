extern crate jit;
#[no_link] #[macro_use]
extern crate jit_macros;
static TEXT:&'static str = "Hello, world!";

extern fn bottles(n: u32) {
	println!("{} bottles sitting on the shelf", n);
}
fn main() {
    use jit::*;
    let mut ctx = Context::<()>::new();
    let b_sig = get::<fn(u32)>();
    jit_func!(&mut ctx, func, fn() -> () {
    	let i = Val::new(&func, &get::<u32>());
    	func.insn_store(i, func.insn_of(10u32));
    	func.build_while(|| func.insn_gt(i, func.insn_of(0u32)), || {
			func.insn_call_native1(Some("bottles"), bottles, &b_sig, [i], flags::NO_THROW);
			func.insn_store(i, func.insn_sub(i, func.insn_of(1u32)));
    	});
        func.insn_default_return();
    }, func());
}
