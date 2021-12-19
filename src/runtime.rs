use std::mem::size_of;
use std::slice::from_raw_parts;

use crate::bytecode::*;

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

// note: スタックサイズ: Byte 単位
pub struct Interpreter {
    bytecode_origin: *mut u8,
    bytecode_top: *mut u8,
    bytecode_len: usize,
    bytecode_counter: usize,
    stack_origin: *mut u8,
    stack_top: *mut u8,
    stack_counter: usize,
    max_stack_size: usize,
    stack_addition_size: usize,
}

impl Interpreter {
    pub unsafe fn launch(bytecode: Bytecode, max_stack_size: usize, stack_addition_size: usize) -> RuntimeResult<Interpreter> {
        // note: スタックの初期サイズが最大サイズを超えている場合
        if stack_addition_size > max_stack_size {
            return Err(RuntimeError::StackOverflow {});
        }

        let mut bytecode_bytes = bytecode.into_vec();
        let bytecode_origin = bytecode_bytes.as_mut_ptr();
        let stack_origin = malloc(size_of::<u8>() * stack_addition_size * 1000) as *mut u8;

        let interpreter = Interpreter {
            bytecode_origin: bytecode_origin,
            bytecode_top: bytecode_origin.add(128),
            bytecode_len: bytecode_bytes.len(),
            bytecode_counter: 128,
            stack_origin: stack_origin,
            stack_top: stack_origin,
            stack_counter: 0,
            max_stack_size: max_stack_size * 1000,
            stack_addition_size: stack_addition_size,
        };

        return Ok(interpreter);
    }

    pub unsafe fn run(&mut self) -> RuntimeResult<()> {
        macro_rules! add {
            ($ty:ty, $f:ident) => {
                {
                    let v1 = *self.pop::<$ty>();
                    let v2 = *self.pop::<$ty>();
                    let (v3, is_overflowed) = v1.$f(v2);

                    if is_overflowed {
                        return Err(RuntimeError::ArithmeticOverflow {});
                    }

                    self.push::<$ty>(v3);
                }
            };
        }

        macro_rules! push_next {
            ($ty:ty) => {
                {
                    let value = *self.next::<$ty>();
                    self.push::<$ty>(value);
                }
            };
        }

        loop {
            let opcode = *self.next::<u8>();

            println!("opcode 0x{:0x} {:?}", opcode, self.to_bytecode_vec());

            match opcode {
                // RET
                0x00 => {
                    let ret_value = self.pop::<u64>();
                    println!("\treturn");
                    println!("\targument: {:0x}", ret_value);
                },
                // EXIT
                0x01 => break,
                // PUSH_32
                0x02 => push_next!(u32),
                // PUSH_64
                0x03 => push_next!(u64),
                // POP_32
                0x04 => {
                    let _ = self.pop::<u32>();
                    ()
                },
                // POP_64
                0x05 => {
                    let _ = self.pop::<u64>();
                    ()
                },
                // IADD_32
                0x06 => add!(u32, overflowing_add),
                // IADD_64
                0x07 => add!(u64, overflowing_add),
                _ => return Err(RuntimeError::UnknownOpcode {}),
            }
        }

        println!();
        println!("stack result: {:?}", from_raw_parts(self.stack_origin, self.stack_counter));
        println!();

        return Ok(());
    }

    unsafe fn to_bytecode_vec(&mut self) -> Vec<u8> {
        return from_raw_parts(self.stack_origin, self.stack_counter).to_vec();
    }

    unsafe fn next<T>(&mut self) -> &T {
        if self.bytecode_counter > self.bytecode_len {
            panic!("unexpected end of bytecode");
        }

        let size = size_of::<T>();
        self.bytecode_counter += size;

        let tmp_bytecode = self.bytecode_top as *mut T;
        let value = &*tmp_bytecode;
        self.bytecode_top = tmp_bytecode as *mut u8;

        self.bytecode_top = self.bytecode_top.add(size);

        return value;
    }

    #[inline(always)]
    unsafe fn push<T>(&mut self, value: T) {
        let value_size = size_of::<T>();

        if self.stack_counter + value_size > self.max_stack_size {
            panic!("stack access violation (stack overflow)");
        }

        self.stack_counter += value_size;

        let tmp_stack = self.stack_top as *mut T;
        *tmp_stack = value;

        self.stack_top = self.stack_top.add(size_of::<T>());
    }

    #[inline(always)]
    unsafe fn pop<T>(&mut self) -> &T {
        let value_size = size_of::<T>();

        if self.stack_counter < value_size {
            panic!("stack access violation");
        }

        self.stack_counter -= value_size;

        self.stack_top = self.stack_top.sub(size_of::<T>());
        let pop_value = self.stack_top as *mut T;
        return &*pop_value;
    }
}

impl Drop for Interpreter {
    fn drop(&mut self) {
        unsafe {
            free(self.stack_origin as *mut c_void);
        }
    }
}
