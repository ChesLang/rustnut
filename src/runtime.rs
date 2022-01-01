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
    CallStackOverflow,
    StackAccessViolation,
    CallStackAccessViolation,
    ArithmeticOverflow,
    DivideByZero,
    Unknown,
}

impl Display for ExitStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ExitStatus::Success => "SUCCESS",
            ExitStatus::UnknownOpcode => "UNKNOWN_OPCODE",
            ExitStatus::BytecodeAccessViolation => "BYTECODE_ACCESS_VIOLATION",
            ExitStatus::StackOverflow => "STACK_OVERFLOW",
            ExitStatus::CallStackOverflow => "CALL_STACK_OVERFLOW",
            ExitStatus::StackAccessViolation => "STACK_ACCESS_VIOLATION",
            ExitStatus::CallStackAccessViolation => "CALL_STACK_ACCESS_VIOLATION",
            ExitStatus::ArithmeticOverflow => "ARITHMETIC_OVERFLOW",
            ExitStatus::DivideByZero => "DIVIDE_BY_ZERO",
            ExitStatus::Unknown => "UNKNOWN",
        };

        return write!(f, "{}", s);
    }
}

impl From<u32> for ExitStatus {
    fn from(v: u32) -> ExitStatus {
        return match v {
            0 => ExitStatus::Success,
            1 => ExitStatus::UnknownOpcode,
            2 => ExitStatus::BytecodeAccessViolation,
            3 => ExitStatus::StackOverflow,
            4 => ExitStatus::CallStackOverflow,
            5 => ExitStatus::StackAccessViolation,
            6 => ExitStatus::CallStackAccessViolation,
            7 => ExitStatus::ArithmeticOverflow,
            8 => ExitStatus::DivideByZero,
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

        let pool_offset = 128usize;
        let mut pool_ptr = bytecode_ptr.add(pool_offset);

        let entry_point_func_index = *(bytecode_ptr.add(*(pool_ptr as *mut usize) + 1) as *mut usize);
        let entry_point_pc = *(bytecode_ptr.add(entry_point_func_index) as *mut usize);
        let mut inst_ptr = bytecode_ptr.add(entry_point_pc);

        let max_stack_size = 1024usize;
        let mut stack_ptr = malloc(max_stack_size) as *mut c_void;

        // note: Exit Status
        let es;
        // note: Stack Pointer
        let mut sp = 0usize;
        // note: Program Counter
        let mut pc = entry_point_pc;
        // note: Pool Pointer
        let mut pp = pool_offset;

        println!("{}", pc);

        // note: エントリポイント用のコールスタック要素をプッシュ

        /*
        // * リターンアドレス
        let tmp_ptr = call_stack_ptr as *mut usize;
        *tmp_ptr = bytecode_len - 1;
        call_stack_ptr = call_stack_ptr.add(size_of::<usize>());

        // * 引数サイズ
        let tmp_ptr = call_stack_ptr as *mut u32;
        *tmp_ptr = 0;
        call_stack_ptr = call_stack_ptr.add(size_of::<u32>());

        // * スタックポインタ
        let tmp_ptr = call_stack_ptr as *mut u32;
        *tmp_ptr = 0;
        call_stack_ptr = call_stack_ptr.add(size_of::<u32>());

        csp += size_of::<usize>() + size_of::<u32>() * 2;
        */

        macro_rules! raw_ptr_to_string {
            ($ptr:expr, $size:expr) => {
                {
                    let mut i = 0usize;
                    let bytes = from_raw_parts($ptr as *const u8, $size).to_vec();

                    if bytes.len() != 0 {
                        bytes.iter().map(|v| {
                            let div = if i != 0 && i % 8 == 0 { "|\n" } else { "" };
                            i += 1;

                            let zero = if format!("{:0x}", v).len() == 1 { "0" } else { "" };

                            format!("{}{}{:0x} ", div, zero, v)
                        }).collect::<Vec<String>>().join("")
                    } else {
                        "<empty>".to_string()
                    }
                }
            };
        }

        'operator: loop {
            macro_rules! next {
                ($ptr:expr, $curr_pos:expr, $ty:ty, $size:expr, $err_status:expr) => {
                    {
                        let value_size = size_of::<$ty>();

                        if $curr_pos + value_size > $size {
                            exit!($err_status);
                        }

                        let tmp_ptr = $ptr as *mut $ty;
                        let value = *tmp_ptr;
                        $ptr = (tmp_ptr as *mut c_void).add(value_size);
                        $curr_pos += value_size;

                        value
                    }
                };
            }

            macro_rules! next_prg {
                ($ty:ty) => {
                    next!(inst_ptr, pc, $ty, bytecode_len, ExitStatus::BytecodeAccessViolation)
                };
            }

            macro_rules! exit {
                ($status_kind:expr) => {
                    {
                        es = $status_kind as u32;
                        break 'operator;
                    }
                };
            }

            let tmp_pc = pc;
            let opcode = next_prg!(u8);
            let opcode_kind = Opcode::from(opcode);

            println!("{}", format!("{} (0x{:0x} at 0x{:0x})", opcode_kind.to_string().to_uppercase(), opcode, tmp_pc).blue());
            println!("{}", raw_ptr_to_string!(stack_ptr.sub(sp), sp).bright_black());
            println!();

            match opcode_kind {
                Opcode::Nop => (),
                Opcode::Exit => exit!(ExitStatus::Success),
                Opcode::Unknown => exit!(ExitStatus::UnknownOpcode),
            }
        }

        let exit_status_msg = format!("exit status 0x{:0x} ({})", es, ExitStatus::from(es).to_string());

        println!("{}", if es == 0 {
            exit_status_msg.on_bright_black()
        } else {
            exit_status_msg.on_red()
        });

        free(stack_ptr.sub(sp));

        return ExitStatus::from(es);
    }
}
