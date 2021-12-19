use std::fmt::{Formatter, Display};
use std::mem::size_of;
use std::slice::from_raw_parts;

use crate::bytecode::*;

use colored::*;

use libc::{c_void, malloc, free};

use rustnutlib::*;
use rustnutlib::console::*;
use rustnutlib::file::*;

pub enum RuntimeError {
    Unknown {},
    FileError { err: FileError },
    ArithmeticOverflow {},
    IndexOutOfBytecodeRange {},
    InvalidHeaderSize {},
    InvalidMagicNumber {},
    InvalidTermination {},
    StackOverflow {},
    UnknownOpcode {},
}

impl ConsoleLogger for RuntimeError {
    fn get_log(&self) -> ConsoleLog {
        return match self {
            RuntimeError::Unknown {} => log!(Error, "unknown error"),
            RuntimeError::FileError { err } => err.get_log(),
            RuntimeError::ArithmeticOverflow {} => log!(Error, &format!("arithmetic overflow")),
            RuntimeError::IndexOutOfBytecodeRange {} => log!(Error, &format!("index out of bytecode range")),
            RuntimeError::InvalidHeaderSize {} => log!(Error, &format!("invalid header size")),
            RuntimeError::InvalidMagicNumber {} => log!(Error, &format!("invalid magic number")),
            RuntimeError::InvalidTermination {} => log!(Error, &format!("invalid termination")),
            RuntimeError::StackOverflow {} => log!(Error, &format!("stack overflow")),
            RuntimeError::UnknownOpcode {} => log!(Error, &format!("unknown opcode")),
        };
    }
}

pub type RuntimeResult<T> = Result<T, RuntimeError>;

pub struct DynamicHeap {
    origin_ptr: *mut c_void,
    top_ptr: *mut c_void,
    size: usize,
    max_size: usize,
    additional_size: usize,
    counter: usize,
    offset: usize,
    is_freed: bool,
}

impl DynamicHeap {
    // arg: スタックサイズ: Byte 単位
    pub unsafe fn new(max_size: usize, additional_size: usize) -> DynamicHeap {
        let size = size_of::<u8>() * additional_size;
        let raw_ptr = malloc(size);

        return DynamicHeap::from(raw_ptr, size, 0, max_size, additional_size);
    }

    pub unsafe fn from(raw_ptr: *mut c_void, size: usize, offset: usize, max_size: usize, additional_size: usize) -> DynamicHeap {
        if offset > size || additional_size > max_size {
            panic!("{}", "stack overflow on dynamic heap".on_red());
        }

        return DynamicHeap {
            origin_ptr: raw_ptr,
            top_ptr: raw_ptr.add(offset),
            size: size,
            max_size: max_size,
            additional_size: additional_size,
            counter: offset,
            offset: offset,
            is_freed: false,
        }
    }

    fn check_freed(&self) {
        if self.is_freed {
            panic!("{}", "heap access violation: use of freed area".on_red());
        }
    }

    unsafe fn free(&mut self) {
        self.check_freed();

        free(self.origin_ptr);
        self.is_freed = true;
    }

    unsafe fn next<T>(&mut self) -> &T {
        self.check_freed();

        let value_size = size_of::<T>();

        if self.counter + value_size > self.size {
            panic!("{}", "dynamic heap access violation".on_red());
        }

        self.counter += value_size;

        let tmp_ptr = self.top_ptr as *mut T;
        let value = &*tmp_ptr;
        self.top_ptr = (tmp_ptr as *mut c_void).add(value_size);

        return value;
    }

    pub unsafe fn pop<T>(&mut self) -> &T {
        self.check_freed();

        let value_size = size_of::<T>();

        if self.counter < value_size {
            panic!("{}", "stack access violation".on_red());
        }

        self.counter -= value_size;

        self.top_ptr = self.top_ptr.sub(size_of::<T>());
        let pop_value = self.top_ptr as *mut T;
        return &*pop_value;
    }

    pub unsafe fn push<T>(&mut self, value: T) {
        self.check_freed();

        let value_size = size_of::<T>();

        if self.counter + value_size > self.max_size {
            panic!("{}", "stack access violation (stack overflow)".on_red());
        }

        let tmp_stack = self.top_ptr as *mut T;
        *tmp_stack = value;

        self.counter += value_size;
        self.top_ptr = self.top_ptr.add(size_of::<T>());
    }

    pub fn len(&self) -> usize {
        self.check_freed();

        return self.counter;
    }

    pub unsafe fn to_bytes(&self) -> Vec<u8> {
        self.check_freed();

        return from_raw_parts(self.origin_ptr as *const u8, self.counter)[self.offset..].to_vec();
    }
}

impl Display for DynamicHeap {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        unsafe {
            let mut i = 0usize;
            let bytes = self.to_bytes();

            let s = if bytes.len() != 0 {
                bytes.iter().map(|v| {
                    let div = if i != 0 && i % 8 == 0 { "|\n" } else { "" };
                    i += 1;
                    format!("{}{:0x} ", div, v)
                }).collect::<Vec<String>>().join("")
            } else {
                "<empty>".to_string()
            };

            return write!(f, "{}", s);
        }
    }
}

pub enum OpcodeKind {
    Unknown,
    Ret,
    Exit,
    Push32,
    Push64,
    Pop32,
    Pop64,
    IAdd32,
    IAdd64,
}

impl Display for OpcodeKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            OpcodeKind::Unknown => "unknown",
            OpcodeKind::Ret => "ret",
            OpcodeKind::Exit => "exit",
            OpcodeKind::Push32 => "push_32",
            OpcodeKind::Push64 => "push_64",
            OpcodeKind::Pop32 => "pop_32",
            OpcodeKind::Pop64 => "pop_64",
            OpcodeKind::IAdd32 => "iadd_32",
            OpcodeKind::IAdd64 => "iadd_64",
        };

        return write!(f, "{}", s);
    }
}

impl From<u8> for OpcodeKind {
    fn from(value: u8) -> OpcodeKind {
        return match value {
            0x00 => OpcodeKind::Ret,
            0x01 => OpcodeKind::Exit,
            0x02 => OpcodeKind::Push32,
            0x03 => OpcodeKind::Push64,
            0x04 => OpcodeKind::Pop32,
            0x05 => OpcodeKind::Pop64,
            0x06 => OpcodeKind::IAdd32,
            0x07 => OpcodeKind::IAdd64,
            _ => OpcodeKind::Unknown,
        };
    }
}

pub struct Interpreter {
    bytecode: DynamicHeap,
    stack: DynamicHeap,
}

impl Interpreter {
    pub unsafe fn launch(bytecode: Bytecode, max_stack_size: usize, stack_addition_size: usize) -> RuntimeResult<Interpreter> {
        let mut bytecode_bytes = bytecode.into_vec();
        let bytecode_heap = DynamicHeap::from(bytecode_bytes.as_mut_ptr() as *mut c_void, bytecode_bytes.len(), 128, 1024 * 1024, 1024);

        let stack = DynamicHeap::new(max_stack_size, stack_addition_size);

        let interpreter = Interpreter {
            bytecode: bytecode_heap,
            stack: stack,
        };

        return Ok(interpreter);
    }

    pub unsafe fn run(&mut self) -> RuntimeResult<()> {
        macro_rules! calc {
            ($ty:ty, $f:ident) => {
                {
                    let v1 = *self.stack.pop::<$ty>();
                    let v2 = *self.stack.pop::<$ty>();
                    let (v3, is_overflowed) = v1.$f(v2);

                    println!("operand {}, {}", v1, v2);

                    if is_overflowed {
                        return Err(RuntimeError::ArithmeticOverflow {});
                    }

                    self.stack.push::<$ty>(v3);
                }
            };
        }

        macro_rules! push_next {
            ($ty:ty) => {
                {
                    let value = *self.bytecode.next::<$ty>();
                    self.stack.push::<$ty>(value);
                }
            };
        }

        loop {
            let opcode = *self.bytecode.next::<u8>();
            let opcode_kind = OpcodeKind::from(opcode);

            println!("{}", format!("{} (0x{:0x})", opcode_kind, opcode).blue());
            println!("{}", self.stack.to_string().bright_black());
            println!();

            match opcode_kind {
                OpcodeKind::Ret => {
                    let ret_value = self.stack.pop::<u32>();
                    println!("ret_value {:0x}", ret_value);
                },
                OpcodeKind::Exit => break,
                OpcodeKind::Push32 => push_next!(u32),
                OpcodeKind::Push64 => push_next!(u64),
                OpcodeKind::Pop32 => {
                    let _ = self.stack.pop::<u32>();
                    ()
                },
                OpcodeKind::Pop64 => {
                    let _ = self.stack.pop::<u64>();
                    ()
                },
                OpcodeKind::IAdd32 => calc!(u32, overflowing_add),
                OpcodeKind::IAdd64 => calc!(u64, overflowing_add),
                _ => return Err(RuntimeError::UnknownOpcode {}),
            }

            println!();
        }

        if self.stack.len() != 0 {
            panic!("{}", "unconsumed stack element".on_red());
        }

        return Ok(());
    }
}

impl Drop for Interpreter {
    fn drop(&mut self) {
        unsafe {
            self.stack.free();
        }
    }
}
