extern crate hyper;
extern crate jit;

use hyper::client::Client;
use jit::*;
use std::cell::RefCell;
use std::io::prelude::*;
use std::io;
use std::fs::File;
use std::iter::Peekable;
use std::mem;
use std::env;
use std::rc::Rc;
use std::os::raw;

pub type Cell = u8;
extern {
    fn putchar(c: raw::c_int) -> ();
    fn getchar() -> raw::c_int;
}

/// Print a character to stdout.
extern fn put_char(c: Cell) {
    unsafe { putchar(c as raw::c_int) }
}
/// Read a character from stdout.
extern fn get_char() -> Cell {
    unsafe { getchar() as Cell }
}

const PROMPT:&'static str = "> ";
type WrappedLoop<'a> = Rc<RefCell<Loop<'a>>>;
/// This type represents a single pair of '[' and ']' in brainfuck code
struct Loop<'a> {
    start: Label<'a>,
    end: Label<'a>,
    parent: Option<WrappedLoop<'a>>
}

impl<'a> Loop<'a> {
    /// Construct a new loop in `func` as a subloop of `current_loop`
    fn new(func: &'a UncompiledFunction, current_loop: Option<WrappedLoop<'a>>) -> Loop<'a> {
        let mut new_loop = Loop {
            start: Label::new(func),
            end: Label::new(func),
            parent: current_loop
        };
        func.insn_label(&mut new_loop.start);
        new_loop
    }
    /// Generate the appropriate IR to end the loop in the function `func`
    fn end(&mut self, func: &'a UncompiledFunction) -> Option<WrappedLoop<'a>> {
        // Branch back to the start of the loop
        func.insn_branch(&mut self.start);
        // Place the label for the end of the loop.
        func.insn_label(&mut self.end);
        // Set `parent` to `None`.
        let mut parent = None;
        mem::swap(&mut parent, &mut self.parent);
        parent
    }
}

/// Count the number of times the character `curr` is repeated in `code`,
/// assuming that the last character to be yielded by the iterator `code`
/// was also `curr`. 
fn count<'a, I>(code: &mut Peekable<I>, curr:char) -> usize where I:Iterator<Item=char> {
    let mut amount = 1;
    while code.peek() == Some(&curr) {
        amount += 1;
        code.next();
    }
    amount
}

/// Compile the brainfuck code `code` into IR in the function `func`.
fn compile<'a>(func: &UncompiledFunction, code: &str) {
    // get the LibJIT equivalents of essential types.
    let cell_t = get::<Cell>();
    let cell_size = mem::size_of::<Cell>();
    let putchar_sig = get::<fn(Cell)>();
    let getchar_sig = get::<fn() -> Cell>();
    let ref data = func[0];
    let mut current_loop = None;
    let mut code = code.chars().peekable();
    while let Some(c) = code.next() {
        match c {
            '>' => {
                let amount = count(&mut code, c);
                let new_value = data + func.insn_of(cell_size * amount);
                func.insn_store(data, new_value);
            },
            '<' => {
                let amount = count(&mut code, c);
                let new_value = data - func.insn_of(cell_size * amount);
                func.insn_store(data, new_value);
            },
            '+' => {
                let amount = count(&mut code, c);
                let mut value = func.insn_load_relative(data, 0, &cell_t);
                value = value + func.insn_of(cell_size * amount);
                value = func.insn_convert(value, &cell_t, false);
                func.insn_store_relative(data, 0, value)
            },
            '-' => {
                let amount = count(&mut code, c);
                let mut value = func.insn_load_relative(data, 0, &cell_t);
                value = value - func.insn_of(cell_size * amount);
                value = func.insn_convert(value, &cell_t, false);
                func.insn_store_relative(data, 0, value)
            },
            '.' => {
                let value = func.insn_load_relative(data, 0, &cell_t);
                func.insn_call_native1(Some("putchar"), put_char, &putchar_sig, [value], flags::NO_THROW);
            },
            ',' => {
                let value = func.insn_call_native0(Some("getchar"), get_char, &getchar_sig, flags::NO_THROW);
                func.insn_store_relative(data, 0, value);
            },
            '[' => {
                let wrapped_loop = Rc::new(RefCell::new(Loop::new(func, current_loop)));
                let tmp = func.insn_load_relative(data, 0, &cell_t);
                {
                    let mut borrow = wrapped_loop.borrow_mut();
                    func.insn_branch_if_not(tmp, &mut borrow.end);
                }
                current_loop = Some(wrapped_loop);
            },
            ']' => {
                current_loop = if let Some(ref inner_loop) = current_loop {
                    let mut borrow = inner_loop.borrow_mut();
                    borrow.end(func)
                } else {
                    None
                }
            },
            _ => ()
        }
    };
    func.insn_default_return();
}
/// Run the brainfuck code `code` by temporarily constructing a new function
/// in `ctx`
fn run(ctx: &mut Context, code: &str) {
    let sig = get::<fn(*const Cell)>();
    // make a new function for the code
    let func = UncompiledFunction::new(ctx, &sig);
    // generate the IR for the code
    compile(&func, code);
    // compile the code and run it
    UncompiledFunction::compile(func).with(|func:extern fn(*mut Cell)| {
        let mut data: [Cell; 3000] = unsafe { mem::zeroed() };
        func(data.as_mut_ptr());
    });
}
/// Read the contents of `file` as UTF-8 and run it as brainfuck code using
/// the context `ctx`
fn open_file(mut ctx: &mut Context, file: &str) {
    let mut text = String::new();
    // read `file` to `text`
    File::open(file).unwrap().read_to_string(&mut text).unwrap();
    // run `text`
    run(&mut ctx, text.trim());
}
fn main() {
    // make a new context to make functions on
    let mut ctx = Context::new();
    let mut args = env::args().skip(1);
    // if an argument was given
    if let Some(ref script) = args.next() {
        // assume it's a file and run it as code
        open_file(&mut ctx, script);
    } else {
        // get i/o streams
        let input = io::stdin();
        let mut output = io::stdout();
        // buffer for temporary strings
        let mut line = String::new();
        loop {
            // print out a prompt for the REPL
            output.write(PROMPT.as_bytes()).unwrap();
            output.flush().unwrap();
            // read a line of input into `line`
            input.read_line(&mut line).unwrap();
            // special cases
            match line.trim() {
                "file" => {
                    println!("Please enter a file path to open:");
                    input.read_line(&mut line).unwrap();
                    open_file(&mut ctx, &line);
                },
                "url" => {
                    println!("Please enter a URL to open:");
                    input.read_line(&mut line).unwrap();
                    let client = Client::new();
                    let mut res = client.get(&line).send().unwrap();
                    res.read_to_string(&mut line).unwrap();
                    run(&mut ctx, &line);
                },
                _ => run(&mut ctx, &line)
            }
            output.write("\n".as_bytes()).unwrap();
        }
    }
}
