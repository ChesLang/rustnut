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
    Unknown,
}

impl Display for ExitStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ExitStatus::Success => "SUCCESS",
            ExitStatus::UnknownOpcode => "UNKNOWN_OPCODE",
            ExitStatus::BytecodeAccessViolation => "BYTECODE_ACCESS_VIOLATION",
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
            _ => ExitStatus::Unknown,
        };
    }
}

pub enum Opcode {
    Unknown,
    Nop,
    Exit,
}

impl Display for Opcode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Opcode::Unknown => "unknown",
            Opcode::Nop => "nop",
            Opcode::Exit => "exit",
        };

        return write!(f, "{}", s);
    }
}

impl From<u8> for Opcode {
    fn from(value: u8) -> Opcode {
        return match value {
            0x00 => Opcode::Nop,
            0x01 => Opcode::Exit,
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

        let stack_ptr = malloc(1024) as *mut c_void;
        let call_stack_ptr = malloc(1024) as *mut c_void;

        // note: Exit Status
        let mut es = 0i32;
        // note: Stack Pointer
        let sp = 0usize;
        // note: Program Counter
        let mut pc = entry_point_index;

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

        loop {
            let tmp_pc = pc;
            let opcode = next_bytecode!(u8);
            let opcode_kind = Opcode::from(opcode);

            println!("{}", format!("{} (0x{:0x} at 0x{:0x})", opcode_kind, opcode, tmp_pc).blue());
            println!("{}", raw_ptr_to_string!(stack_ptr, sp).bright_black());
            println!();

            match opcode_kind {
                Opcode::Nop => (),
                Opcode::Exit => break,
                Opcode::Unknown => {
                    es = ExitStatus::UnknownOpcode as i32;
                    break;
                },
            }
        }

        let exit_status_msg = format!("exit status 0x{:0x} ({})", es, ExitStatus::from(es).to_string());
        println!("{}", if es == 0 {
            exit_status_msg.on_bright_black()
        } else {
            exit_status_msg.on_red()
        });

        // if self.stack.counter != 0 {
        //     println!("unconsumed stack element(s):");
        //     println!("{}", self.stack);
        //     panic!("{}", "unconsumed stack element(s)".on_red());
        // }

        free(stack_ptr);
        free(call_stack_ptr);

        return ExitStatus::from(es);
    }
}
