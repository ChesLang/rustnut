use std::mem::size_of;

use crate::bytecode::*;

use libc::*;

use rustnutlib::*;
use rustnutlib::console::*;
use rustnutlib::file::*;

pub enum RuntimeError {
    Unknown {},
    FileError { err: FileError },
    IndexOutOfBytecodeRange {},
    InvalidHeaderSize {},
    InvalidMagicNumber {},
    StackOverflow {},
}

impl ConsoleLogger for RuntimeError {
    fn get_log(&self) -> ConsoleLog {
        return match self {
            RuntimeError::Unknown {} => log!(Error, "unknown error"),
            RuntimeError::FileError { err } => err.get_log(),
            RuntimeError::IndexOutOfBytecodeRange {} => log!(Error, &format!("index out of bytecode range")),
            RuntimeError::InvalidHeaderSize {} => log!(Error, &format!("invalid header size")),
            RuntimeError::InvalidMagicNumber {} => log!(Error, &format!("invalid magic number")),
            RuntimeError::StackOverflow {} => log!(Error, &format!("stack overflow")),
        };
    }
}

pub type RuntimeResult<T> = Result<T, RuntimeError>;

pub struct Interpreter {}

impl Interpreter {
    // arg: max_stack_size: KB 単位
    pub unsafe fn run(bytecode: &Bytecode, max_stack_size: usize, stack_addition_size: usize) -> RuntimeResult<()> {
        // note: スタックの初期サイズが最大サイズを超えている場合
        if stack_addition_size > max_stack_size {
            return Err(RuntimeError::StackOverflow {});
        }

        let mut stack = malloc(size_of::<u8>() * stack_addition_size * 1000) as *mut u64;
        println!("add {}", *stack.add(10000000));
        *stack = 0xccaa;

        println!("value {:0x}", *stack);

        free(stack as *mut c_void);

        let array: [i32; 6] = [11, 22, 33, 44, 55, 66];
        let slice: &[i32] = &array;
        // C: int32_t *rawptr = &slice[0];
        let rawptr: *const i32 = &slice[0] as *const i32;  // <= 生ポインタ取得
        println!("*{:?} = {}", rawptr, *rawptr);   // <= *0x7ffee8de5020 = 11
        return Ok(());
    }
}
