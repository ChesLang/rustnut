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
            _ => ExitStatus::Unknown,
        };
    }
}

pub enum Opcode {
    Unknown,
    Nop,
    Exit,
    Invoke,
    Ret,
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
            Opcode::Invoke => "invoke",
            Opcode::Ret => "ret",
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
            0x02 => Opcode::Invoke,
            0x03 => Opcode::Ret,
            0x04 => Opcode::Push32,
            0x05 => Opcode::Push64,
            0x06 => Opcode::Pop32,
            0x07 => Opcode::Pop64,
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

        let mut link_area_ptr = bytecode_ptr;
        let link_area_offset = 128usize;
        link_area_ptr = link_area_ptr.add(link_area_offset);
        let link_element_size = 16;

        let entry_point_index = *(link_area_ptr as *mut usize);
        bytecode_ptr = bytecode_ptr.add(entry_point_index);

        let max_stack_size = 1024usize;
        let mut stack_ptr = malloc(max_stack_size) as *mut c_void;
        let mut call_stack_ptr = malloc(max_stack_size) as *mut c_void;

        // todo: 接頭辞 r_
        // note: Exit Status
        let es;
        // note: Stack Pointer
        let mut sp = 0usize;
        // note: Call Stack Pointer
        let mut csp = 0usize;
        // note: Program Counter
        let mut pc = entry_point_index;
        // note: Link Area Pointer
        let mut lap = link_area_offset;

        // note: エントリポイント用のコールスタック要素をプッシュ

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
            macro_rules! jump_to {
                ($ptr:expr, $curr_pos:expr, $jump_to:expr, $size:expr, $err_status:expr) => {
                    {
                        if $jump_to >= $size {
                            exit!($err_status);
                        }

                        $ptr = $ptr.offset($jump_to as isize - $curr_pos as isize);
                        $curr_pos = $jump_to;
                    }
                };
            }

            macro_rules! jump_to_link_elem {
                ($link_num:expr) => {
                    jump_to!(link_area_ptr, lap, link_area_offset + $link_num * link_element_size, bytecode_len, ExitStatus::BytecodeAccessViolation as u32)
                };
            }

            macro_rules! jump_bytecode_to {
                ($address:expr) => {
                    jump_to!(bytecode_ptr, pc, $address, bytecode_len, ExitStatus::BytecodeAccessViolation as u32)
                };
            }

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

            macro_rules! next_bytecode {
                ($ty:ty) => {
                    next!(bytecode_ptr, pc, $ty, bytecode_len, ExitStatus::BytecodeAccessViolation as u32)
                };
            }

            macro_rules! next_link_area {
                ($ty:ty) => {
                    next!(link_area_ptr, lap, $ty, bytecode_len, ExitStatus::BytecodeAccessViolation as u32)
                };
            }

            macro_rules! top {
                ($ptr:expr, $counter:expr, $ty:ty, $err_status:expr) => {
                    {
                        let value_size = size_of::<$ty>();

                        if $counter < value_size {
                            exit!($err_status);
                        }

                        *($ptr as *mut $ty).sub(1)
                    }
                };
            }

            macro_rules! call_stack_top {
                ($ty:ty) => {
                    top!(call_stack_ptr, csp, $ty, ExitStatus::CallStackOverflow as u32)
                };
            }

            macro_rules! push {
                ($ptr:expr, $curr_pos:expr, $ty:ty, $value:expr, $size:expr, $err_status:expr) => {
                    {
                        let value_size = size_of::<$ty>();

                        if $curr_pos + value_size > $size {
                            exit!($err_status);
                        }

                        let tmp_ptr = $ptr as *mut $ty;
                        *tmp_ptr = $value;

                        $curr_pos += value_size;
                        $ptr = $ptr.add(value_size);
                    }
                };
            }

            macro_rules! push_stack {
                ($ty:ty, $value:expr $(, $count_fsp:expr)?) => {
                    {
                        push!(stack_ptr, sp, $ty, $value, max_stack_size, ExitStatus::StackOverflow as u32);

                        #[allow(unused_assignments, unused_mut)]
                        let mut count = true;
                        $(count = $count_fsp;)?

                        if count {
                            add_fsp!(size_of::<$ty>() as u32);
                        }
                    }
                };
            }

            macro_rules! push_call_stack {
                ($ty:ty, $value:expr) => {
                    push!(call_stack_ptr, csp, $ty, $value, max_stack_size, ExitStatus::CallStackOverflow as u32)
                };
            }

            macro_rules! push_stack_next {
                ($ty:ty) => {
                    {
                        let value = next_bytecode!($ty);
                        push_stack!($ty, value);
                    }
                };
            }

            macro_rules! pop {
                ($ptr:expr, $curr_pos:expr, $ty:ty, $err_status:expr) => {
                    {
                        let value_size = size_of::<$ty>();

                        if $curr_pos < value_size {
                            exit!($err_status);
                        }

                        $curr_pos -= value_size;
                        $ptr = $ptr.sub(value_size);

                        *($ptr as *mut $ty)
                    }
                };
            }

            macro_rules! pop_stack {
                ($ty:ty $(, $count_fsp:expr)?) => {
                    {
                        pop!(stack_ptr, sp, $ty, ExitStatus::StackAccessViolation as u32);

                        #[allow(unused_assignments, unused_mut)]
                        let mut count = true;
                        $(count = $count_fsp;)?

                        if count {
                            sub_fsp!(size_of::<$ty>() as u32);
                        }
                    }
                };
            }

            macro_rules! pop_call_stack {
                ($ty:ty) => {
                    pop!(call_stack_ptr, csp, $ty, ExitStatus::CallStackAccessViolation as u32)
                };
            }

            /// コールスタックのスタックポインタ (fsp) を加算
            macro_rules! add_fsp {
                ($size:expr) => {
                    {
                        let fsp = pop_call_stack!(u32);
                        push_call_stack!(u32, fsp + $size);
                    }
                };
            }

            /// コールスタックのスタックポインタ (fsp) を減算
            macro_rules! sub_fsp {
                ($size:expr) => {
                    let fsp = pop_call_stack!(u32);

                    if fsp < $size {
                        exit!(ExitStatus::StackAccessViolation);
                    }

                    push_call_stack!(u32, fsp - $size);
                };
            }

            macro_rules! invoke {
                () => {
                    {
                        let link_num = next_bytecode!(usize);
                        jump_to_link_elem!(link_num);
                        let ret_addr = pc;
                        let start_addr = next_link_area!(usize);
                        let arg_len = next_link_area!(u32);

                        println!("{}", format!("[link number 0x{:0x} / start at 0x{:0x} / return to 0x{:0x} / {} byte arguments]", link_num, start_addr, ret_addr, arg_len * 4).bright_green().dimmed());
                        println!();

                        println!("{}", "call stack (prev):".bright_black());
                        println!("{}", raw_ptr_to_string!(call_stack_ptr.sub(csp), csp).bright_black());
                        println!();

                        // note: 先に呼び出し元のコールスタックから引数分の fsp を減算
                        sub_fsp!(size_of::<u32>() as u32 * arg_len);

                        // note: コールスタックにリターンアドレスをプッシュ
                        push_call_stack!(usize, ret_addr);

                        if sp < arg_len as usize * size_of::<u32>() {
                            exit!(ExitStatus::StackAccessViolation);
                        }

                        // note: 引数をコールスタックへコピー
                        for i in 0..arg_len as usize {
                            let value = *((stack_ptr as *mut u32).sub(arg_len as usize - i));
                            push_call_stack!(u32, value);
                        }

                        // note: スタックから引数分の要素をポップ; コールスタック操作後のため fsp を変更しない
                        for _ in 0..arg_len {
                            pop_stack!(u32, false);
                        }

                        // note: コールスタックに引数サイズをプッシュ
                        push_call_stack!(u32, arg_len);
                        // note: コールスタックにスタックポインタをプッシュ
                        push_call_stack!(u32, 0);

                        println!("{}", "call stack (modded):".bright_black());
                        println!("{}", raw_ptr_to_string!(call_stack_ptr.sub(csp), csp).bright_black());
                        println!();

                        jump_bytecode_to!(start_addr);
                    }
                };
            }

            macro_rules! ret {
                () => {
                    {
                        let fsp = call_stack_top!(u32);

                        for _ in 0..fsp {
                            pop_stack!(u8);
                        }

                        pop_call_stack!(u32);

                        let arg_len = pop_call_stack!(u32);

                        for _ in 0..arg_len {
                            pop_call_stack!(u32);
                        }

                        let ret_addr = pop_call_stack!(usize);

                        println!("{}", format!("[return to 0x{:0x} / {} arguments / pop {} bytes]", ret_addr, arg_len, fsp).bright_green().dimmed());
                        println!();

                        jump_bytecode_to!(ret_addr);

                        println!("{}", "call stack:".bright_black());
                        println!("{}", raw_ptr_to_string!(call_stack_ptr.sub(csp), csp).bright_black());
                        println!();
                    }
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
            let opcode = next_bytecode!(u8);
            let opcode_kind = Opcode::from(opcode);

            println!("{}", format!("{} (0x{:0x} at 0x{:0x})", opcode_kind.to_string().to_uppercase(), opcode, tmp_pc).blue());
            println!("{}", raw_ptr_to_string!(stack_ptr.sub(sp), sp).bright_black());
            println!();

            match opcode_kind {
                Opcode::Nop => (),
                Opcode::Exit => exit!(ExitStatus::Success),
                Opcode::Invoke => invoke!(),
                Opcode::Ret => ret!(),
                Opcode::Push32 => push_stack_next!(u32),
                Opcode::Push64 => push_stack_next!(u64),
                Opcode::Pop32 => {
                    let _ = pop_stack!(u32);
                },
                Opcode::Pop64 => {
                    let _ = pop_stack!(u64);
                },
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
        free(call_stack_ptr.sub(csp));

        return ExitStatus::from(es);
    }
}
