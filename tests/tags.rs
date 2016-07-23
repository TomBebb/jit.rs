extern crate jit;
use jit::*;

#[test]
fn test_context_tags() {
    let mut ctx = Context::<isize>::new();
    ctx[0] = 3;
    ctx[1] = 33;
    assert_eq!(ctx[0], 3);
    assert_eq!(ctx[1], 33);
}
#[derive(Default, Debug, Eq, PartialEq)]
struct PanicDrop(isize);
impl Drop for PanicDrop {
    fn drop(&mut self) {
        panic!("Dropped {:?}", self)
    }
}
#[test]
#[should_panic]
fn test_context_panic_tags() {
    let mut ctx = Context::<PanicDrop>::new();
    ctx[0] = PanicDrop(7);
}