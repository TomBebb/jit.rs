#![feature(test)]
#[no_link] #[macro_use]
extern crate jit_macros;
extern crate jit;
extern crate test;
use test::Bencher;
use jit::*;

#[bench]
fn bench_gcd(b: &mut Bencher) {
    let mut ctx = Context::<()>::new();
    let sig = get::<fn(usize, usize)>();
    let func = UncompiledFunction::new(&ctx, &sig);
    {
        let (x, y) = (&func[0], &func[1]);
        let flags = flags::NO_THROW | flags::TAIL;
        func.build_if(func.insn_eq(x, y), || func.insn_return(x));
        func.build_if(func.insn_lt(x, y), || {
            let v = func.insn_call(Some("gcd"), &func, None, &[x, y - x], flags);
            func.insn_return(v);
        });
        let temp4 = func.insn_call(Some("gcd"), &func, None, &[x - y, y], flags);
        func.insn_return(temp4);
    };
    let func = UncompiledFunction::compile(func);
    let func = CompiledFunction::to_closure::<(usize, usize), usize>(func);
    b.iter(|| assert_eq!(func(90, 50), 10));
}
#[bench]
fn bench_raw_gcd(b: &mut Bencher) {
    fn gcd(x: usize, y: usize) -> usize {
        if x == y {
            x
        } else if x < y {
            gcd(x, y - x)
        } else {
            gcd(x - y, y)
        }
    }
    b.iter(|| assert_eq!(gcd(90, 50), 10));
}
