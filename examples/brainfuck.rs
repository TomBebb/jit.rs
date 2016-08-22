extern crate hyper;
extern crate jit;

use hyper::client::Client;
use jit::*;
use std::cell::RefCell;
use std::io::prelude::*;
use std::io::{BufReader, BufWriter, self};
use std::fs::File;
use std::iter::Peekable;
use std::mem;
use std::env;
use std::rc::Rc;

pub type Cell = u8;

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
fn count<'a, I>(code: &mut Peekable<I>, curr:char) -> usize where I:Iterator<Item=(usize, char)> {
    let mut amount = 1;
    while code.peek().map(|v| v.1) == Some(curr) {
        amount += 1;
        code.next();
    }
    amount
}

/// Generate the IR for the brainfuck code `code` into the function `func`.
fn generate(func: &UncompiledFunction, code_text: &str) {
    // get the LibJIT equivalents of essential types.
    let cell_t = get::<Cell>();
    let cell_size = mem::size_of::<Cell>();
    let mut data = func.insn_dup(&func[0]);
    let mut current_loop = None;
    let mut code = code_text.chars().enumerate().peekable();
    let input = io::stdin();
    let input = BufReader::new(input.lock());
    let mut input = input.bytes();
    let output = io::stdout();
    let mut output = BufWriter::new(output.lock());
    let mut put_char = move |c: Cell| {
        output.write_all(&[c]).unwrap();
        if c == b'\n' || c == b'\r' {
            output.flush().unwrap();
        }
    };
    let mut get_char = move ||
        input.next().map(Result::unwrap).unwrap_or(0);
    while let Some((offset, c)) = code.next() {
        match c {
            '>' => {
                let amount = count(&mut code, c);
                data += cell_size * amount;
            },
            '<' => {
                let amount = count(&mut code, c);
                data -= cell_size * amount;
            },
            '+' => {
                let amount = count(&mut code, c);
                let mut value = func.insn_load_relative(data, 0, &cell_t);
                value += cell_size * amount;
                value = func.insn_convert(value, &cell_t, false);
                func.insn_store_relative(data, 0, value)
            },
            '-' => {
                let amount = count(&mut code, c);
                let mut value = func.insn_load_relative(data, 0, &cell_t);
                value -= cell_size * amount;
                value = func.insn_convert(value, &cell_t, false);
                func.insn_store_relative(data, 0, value)
            },
            '.' => {
                let value = func.insn_load_relative(data, 0, &cell_t);
                func.insn_call_rust_mut(Some("putchar"), &mut put_char, &[value], flags::NO_THROW);
            },
            ',' => {
                let value = func.insn_call_rust_mut(Some("getchar"), &mut get_char, &[], flags::NO_THROW);
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
                    println!("unmatched ] at offset {}", offset);
                    None
                }
            },
            _ => ()
        }
    };
    if current_loop.is_some() {
        panic!("unmatched [ at offset {}", code_text.len());
    }
    func.insn_default_return();
}
/// Run the brainfuck code `code` by temporarily constructing a new function
/// in `ctx`
fn run(ctx: &mut Context, code: &str) {
    let sig = get::<fn(*const Cell)>();
    // make a new function for the code
    let func = UncompiledFunction::new(ctx, &sig);
    // generate the IR for the code
    generate(&func, code);
    // compile the code and run it
    let func = UncompiledFunction::compile(func);
    let mut data: [Cell; 10000] = unsafe { mem::zeroed() };/*
    let closure: &Fn(*mut Cell) = CompiledFunction::to_closure(func);
    closure(data.as_mut_ptr());*/
    let mut result = ();
    func.apply(&[&data.as_mut_ptr()], &mut result);
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
fn prompt<W, R>(output: &mut W, input: &mut R, mut text: &mut String) -> io::Result<()> where W: Write, R: BufRead {
    text.clear();
    try!(output.write(PROMPT.as_bytes()));
    try!(output.flush());
    input.read_line(text).map(|_| ())
}
fn prompt_for<W, R>(output: &mut W, input: &mut R, mut text: &mut String, openee: &str) -> io::Result<()> where W: Write, R: BufRead {
    try!(output.write(openee.as_bytes()));
    try!(output.write(PROMPT.as_bytes()));
    try!(output.flush());
    input.read_line(&mut text).map(|_| ())
}

fn run_url(mut ctx: &mut Context, url: &str) {
    let client = Client::new();
    let mut code = String::new();
    let mut res = client.get(url).send().unwrap();
    res.read_to_string(&mut code).unwrap();
    run(&mut ctx, &code);
}

fn main() {
    // make a new context to make functions on
    let mut ctx = Context::new();
    let args:Vec<String> = env::args().skip(1).collect();
    match args.get(0).map(String::as_str) {
        Some("file") => open_file(&mut ctx, &args[1]),
        Some("url") => run_url(&mut ctx, &args[1]),
        _ => {
            // get i/o streams
            let mut input = BufReader::new(io::stdin());
            let mut output = io::stdout();
            // buffer for temporary strings
            let mut line = String::new();
            loop {
                // prompt for a line of input
                prompt(&mut output, &mut input, &mut line).unwrap();
                // special cases
                match line.trim() {
                    "file" => {
                        prompt_for(&mut output, &mut input, &mut line, "file").unwrap();
                        open_file(&mut ctx, &line);
                    },
                    "url" => {
                        prompt_for(&mut output, &mut input, &mut line, "URL").unwrap();
                        run_url(&mut ctx, &line);
                    },
                    _ => run(&mut ctx, &line)
                }
                output.write("\n".as_bytes()).unwrap();
            }
        }
    }
}
