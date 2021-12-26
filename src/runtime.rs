use std::fmt::{Formatter, Display};
use std::slice::from_raw_parts;
use std::mem::size_of;

use crate::bytecode::*;

use colored::*;

use libc::{c_void, malloc, free};

pub enum ExitStatus {
    Success,
    UnknownOpcode,
    BytecodeAccessViolation,
    StackOverflow,
    StackAccessViolation,
    UnconsumedStackElement,
    Unknown,
}

impl Display for ExitStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ExitStatus::Success => "SUCCESS",
            ExitStatus::UnknownOpcode => "UNKNOWN_OPCODE",
            ExitStatus::BytecodeAccessViolation => "BYTECODE_ACCESS_VIOLATION",
            ExitStatus::StackOverflow => "STACK_OVERFLOW",
            ExitStatus::StackAccessViolation => "STACK_ACCESS_VIOLATION",
            ExitStatus::UnconsumedStackElement => "UNCONSUMED_STACK_ELEMENT",
            ExitStatus::Unknown => "UNKNOWN",
        };

        return write!(f, "{}", s);
    }
}

impl From<i32> for ExitStatus {
    fn from(v: i32) -> ExitStatus {
        return match v {
            0 => ExitStatus::Success,
            1 => ExitStatus::UnknownOpcode,
            2 => ExitStatus::BytecodeAccessViolation,
            3 => ExitStatus::StackOverflow,
            4 => ExitStatus::StackAccessViolation,
            5 => ExitStatus::UnconsumedStackElement,
            _ => ExitStatus::Unknown,
        };
    }
}

pub enum Opcode {
    Unknown,
    Nop,
    Exit,
    Push32,
    Push64,
    Pop32,
    Pop64,
}

impl Display for Opcode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Opcode::Unknown => "unknown",
            Opcode::Nop => "nop",
            Opcode::Exit => "exit",
            Opcode::Push32 => "push_32",
            Opcode::Push64 => "push_64",
            Opcode::Pop32 => "pop_32",
            Opcode::Pop64 => "pop_64",
        };

        return write!(f, "{}", s);
    }
}

impl From<u8> for Opcode {
    fn from(value: u8) -> Opcode {
        return match value {
            0x00 => Opcode::Nop,
            0x01 => Opcode::Exit,
            0x02 => Opcode::Push32,
            0x03 => Opcode::Push64,
            0x04 => Opcode::Pop32,
            0x05 => Opcode::Pop64,
            _ => Opcode::Unknown,
        };
    }
}

pub struct Interpreter {}

impl Interpreter {
    pub unsafe fn launch(bytecode_bytes: Vec<u8>) -> ExitStatus {
        let bytecode = Bytecode::new(bytecode_bytes);

        if *HEADER_SIZE > bytecode.len() {
            panic!("{}", "invalid header size".on_red());
        }

        if !bytecode.match_bytes(HeaderItem::MagicNumber.get_bytecode_range(), &MAGIC_NUMBER.to_vec()) {
            panic!("{}", "invalid magic number".on_red());
        }

        bytecode.print();
        return Interpreter::run(&mut *bytecode.into_vec());
    }

    unsafe fn run(bytecode_bytes: &mut Vec<u8>) -> ExitStatus {
        let bytecode_len = bytecode_bytes.len();
        let mut bytecode_ptr = bytecode_bytes.as_mut_ptr() as *mut c_void;

        // let link_area_ptr = bytecode_ptr;
        // let link_area_offset = 128;
        // link_area_ptr.add(link_area_offset);

        let entry_point_index = 0xd0usize;
        bytecode_ptr = bytecode_ptr.add(entry_point_index);

        let max_stack_size = 1024;
        let mut stack_ptr = malloc(max_stack_size) as *mut c_void;
        let call_stack_ptr = malloc(max_stack_size) as *mut c_void;

        // note: Exit Status
        let mut es = 0i32;
        // note: Stack Pointer
        let mut sp = 0usize;
        // note: Program Counter
        let mut pc = entry_point_index;

        macro_rules! raw_ptr_to_string {
            ($ptr:expr, $size:expr) => {
                {
                    let mut i = 0usize;
                    let bytes = from_raw_parts($ptr as *const u8, $size).to_vec();

                    if bytes.len() != 0 {
                        bytes.iter().map(|v| {
                            let div = if i != 0 && i % 8 == 0 { "|\n" } else { "" };
                            i += 1;
                            format!("{}{:0x} ", div, v)
                        }).collect::<Vec<String>>().join("")
                    } else {
                        "<empty>".to_string()
                    }
                }
            };
        }

        macro_rules! next_bytecode {
            ($ty:ty) => {
                {
                    let size = size_of::<$ty>();

                    if pc + size > bytecode_len {
                        es = ExitStatus::BytecodeAccessViolation as i32;
                        break;
                    }

                    let tmp_ptr = bytecode_ptr as *mut $ty;
                    let value = *tmp_ptr;
                    bytecode_ptr = (tmp_ptr as *mut c_void).add(size);
                    pc += size;

                    value
                }
            };
        }

        macro_rules! push_next {
            ($ty:ty) => {
                {
                    let value = next_bytecode!($ty);
                    let size = size_of::<$ty>();

                    if sp + size > max_stack_size {
                        es = ExitStatus::StackOverflow as i32;
                        break;
                    }

                    let tmp_stack = stack_ptr as *mut $ty;
                    *tmp_stack = value;

                    sp += size;
                    stack_ptr = stack_ptr.add(size);
                }
            };
        }

        macro_rules! pop {
            ($ty:ty) => {
                {
                    let size = size_of::<$ty>();

                    if sp < size {
                        es = ExitStatus::StackAccessViolation as i32;
                        break;
                    }

                    sp -= size;
                    stack_ptr = stack_ptr.sub(size);

                    *(stack_ptr as *mut $ty)
                }
            };
        }

        loop {
            let tmp_pc = pc;
            let opcode = next_bytecode!(u8);
            let opcode_kind = Opcode::from(opcode);

            println!("{}", format!("{} (0x{:0x} at 0x{:0x})", opcode_kind, opcode, tmp_pc).blue());
            println!("{}", raw_ptr_to_string!(stack_ptr.sub(sp), sp).bright_black());
            println!();

            match opcode_kind {
                Opcode::Nop => (),
                Opcode::Exit => break,
                Opcode::Push32 => push_next!(u32),
                Opcode::Push64 => push_next!(u64),
                Opcode::Pop32 => {
                    let _ = pop!(u32);
                },
                Opcode::Pop64 => {
                    let _ = pop!(u64);
                },
                Opcode::Unknown => {
                    es = ExitStatus::UnknownOpcode as i32;
                    break;
                },
            }
        }

        if sp != 0 {
            println!("unconsumed stack element(s):");
            println!("{}", raw_ptr_to_string!(stack_ptr.sub(sp), sp).bright_black());
            println!();

            // note: 終了コードが 0 でなければ上書きしない
            if es == 0 {
                es = ExitStatus::UnconsumedStackElement as i32;
            }
        }

        let exit_status_msg = format!("exit status 0x{:0x} ({})", es, ExitStatus::from(es).to_string());
        println!("{}", if es == 0 {
            exit_status_msg.on_bright_black()
        } else {
            exit_status_msg.on_red()
        });

        free(stack_ptr.sub(sp));
        free(call_stack_ptr);

        return ExitStatus::from(es);
    }
}
