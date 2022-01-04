use std::fmt::{Formatter, Display};
use std::slice::from_raw_parts;
use std::mem::size_of;

use crate::bytecode::*;

use colored::*;

use libc::{c_void, malloc, free, write};

pub enum ExitStatus {
    Success,
    UnknownOpcode,
    UnknownCallNumber,
    BytecodeAccessViolation,
    StackOverflow,
    StackAccessViolation,
    ArithmeticOverflow,
    DivideByZero,
    Unknown,
}

impl Display for ExitStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ExitStatus::Success => "SUCCESS",
            ExitStatus::UnknownOpcode => "UNKNOWN_OPCODE",
            ExitStatus::UnknownCallNumber => "UNKNOWN_CALL_NUMBER",
            ExitStatus::BytecodeAccessViolation => "BYTECODE_ACCESS_VIOLATION",
            ExitStatus::StackOverflow => "STACK_OVERFLOW",
            ExitStatus::StackAccessViolation => "STACK_ACCESS_VIOLATION",
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
            2 => ExitStatus::UnknownCallNumber,
            3 => ExitStatus::BytecodeAccessViolation,
            4 => ExitStatus::StackOverflow,
            5 => ExitStatus::StackAccessViolation,
            6 => ExitStatus::ArithmeticOverflow,
            7 => ExitStatus::DivideByZero,
            _ => ExitStatus::Unknown,
        };
    }
}

pub enum Opcode {
    Unknown,
    Nop,
    Exit,
    Call,
    Invoke,
    Ret,
    BPush,
    SPush,
    IPush,
    LPush,
    Dup,
    Dup2,
    Pop,
    Pop2,
    Load,
    Load2,
    Store,
    Store2,
    IAdd,
    LAdd,
    ISub,
    LSub,
    IMul,
    LMul,
    IDiv,
    LDiv,
    IEq,
    LEq,
    IOrd,
    LOrd,
    IEqOrd,
    LEqOrd,
    Goto,
    If,
}

impl Display for Opcode {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Opcode::Unknown => "unknown",
            Opcode::Nop => "nop",
            Opcode::Exit => "exit",
            Opcode::Call => "call",
            Opcode::Invoke => "invoke",
            Opcode::Ret => "ret",
            Opcode::BPush => "bpush",
            Opcode::SPush => "spush",
            Opcode::IPush => "ipush",
            Opcode::LPush => "lpush",
            Opcode::Dup => "dup",
            Opcode::Dup2 => "dup2",
            Opcode::Pop => "pop",
            Opcode::Pop2 => "pop2",
            Opcode::Load => "load",
            Opcode::Load2 => "load2",
            Opcode::Store => "store",
            Opcode::Store2 => "store2",
            Opcode::IAdd => "iadd",
            Opcode::LAdd => "ladd",
            Opcode::ISub => "isub",
            Opcode::LSub => "lsub",
            Opcode::IMul => "imul",
            Opcode::LMul => "lmul",
            Opcode::IDiv => "idiv",
            Opcode::LDiv => "ldiv",
            Opcode::IEq => "ieq",
            Opcode::LEq => "leq",
            Opcode::IOrd => "iord",
            Opcode::LOrd => "lord",
            Opcode::IEqOrd => "ieqord",
            Opcode::LEqOrd => "leqord",
            Opcode::Goto => "goto",
            Opcode::If => "if",
        };

        return write!(f, "{}", s);
    }
}

impl From<u8> for Opcode {
    fn from(value: u8) -> Opcode {
        return match value {
            0x00 => Opcode::Nop,
            0x01 => Opcode::Exit,
            0x02 => Opcode::Call,
            0x03 => Opcode::Invoke,
            0x04 => Opcode::Ret,
            0x05 => Opcode::BPush,
            0x06 => Opcode::SPush,
            0x07 => Opcode::IPush,
            0x08 => Opcode::LPush,
            0x09 => Opcode::Dup,
            0x0a => Opcode::Dup2,
            0x0b => Opcode::Pop,
            0x0c => Opcode::Pop2,
            0x0d => Opcode::Load,
            0x0e => Opcode::Load2,
            0x0f => Opcode::Store,
            0x10 => Opcode::Store2,
            0x11 => Opcode::IAdd,
            0x12 => Opcode::LAdd,
            0x13 => Opcode::ISub,
            0x14 => Opcode::LSub,
            0x15 => Opcode::IMul,
            0x16 => Opcode::LMul,
            0x17 => Opcode::IDiv,
            0x18 => Opcode::LDiv,
            0x19 => Opcode::IEq,
            0x1a => Opcode::LEq,
            0x1b => Opcode::IOrd,
            0x1c => Opcode::LOrd,
            0x1d => Opcode::IEqOrd,
            0x1e => Opcode::LEqOrd,
            0x1f => Opcode::Goto,
            0x20 => Opcode::If,
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
        let mut is_init_succeeded = true;
        // note: Exit Status
        let mut es = ExitStatus::Success as u32;

        let bytecode_len = bytecode_bytes.len();
        let bytecode_ptr = bytecode_bytes.as_mut_ptr() as *mut c_void;

        let pool_offset = 128usize;
        let mut pool_ptr = bytecode_ptr.add(pool_offset);

        let entry_point_func_index = *(bytecode_ptr.add(*(pool_ptr as *mut usize)) as *mut usize);
        let entry_point_pc = entry_point_func_index;
        let mut inst_ptr = bytecode_ptr.add(entry_point_pc);

        if entry_point_pc >= bytecode_len {
            is_init_succeeded = false;
            es = ExitStatus::BytecodeAccessViolation as u32;
        }

        let max_stack_size = 1024usize;
        let mut stack_ptr = malloc(max_stack_size) as *mut c_void;

        // note: Stack Pointer
        let mut sp = 0usize;
        // note: Base Pointer
        let mut bp = 0usize;
        // note: Program Counter
        let mut pc = entry_point_pc;
        // note: Pool Pointer
        let mut pp = pool_offset;

        // note: 'operator ブロック外での終了処理
        // fix: 処理が中断されない
        macro_rules! exit {
            ($status_kind:ident) => {
                {
                    es = ExitStatus::$status_kind as u32;
                    is_init_succeeded = false;
                }
            };
        }

        macro_rules! jump_to {
            ($ptr:expr, $curr_pos:expr, $jump_to:expr, $size:expr, $err_status:ident) => {
                {
                    if $jump_to > $size {
                        exit!($err_status);
                    }

                    $ptr = $ptr.offset($jump_to as isize - $curr_pos as isize);
                    $curr_pos = $jump_to;
                }
            };
        }

        macro_rules! jump_prg_to {
            ($index:expr) => {
                jump_to!(inst_ptr, pc, $index, bytecode_len, BytecodeAccessViolation)
            };
        }

        macro_rules! jump_pool_to {
            ($pool_index:expr) => {
                {
                    jump_to!(pool_ptr, pp, pool_offset + $pool_index * size_of::<usize>(), bytecode_len, BytecodeAccessViolation);
                    let value_addr = next_pool!(usize);
                    jump_to!(pool_ptr, pp, value_addr, bytecode_len, BytecodeAccessViolation);
                }
            };
        }

        macro_rules! jump_stack_to {
            ($index:expr) => {
                jump_to!(stack_ptr, sp, $index, max_stack_size, StackAccessViolation)
            };
        }

        macro_rules! push {
            ($ptr:expr, $curr_pos:expr, $ty:ty, $value:expr, $size:expr, $err_status:ident) => {
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

        macro_rules! stack_push {
            ($ty:ty, $value:expr) => {
                push!(stack_ptr, sp, $ty, $value, max_stack_size, StackOverflow)
            };

            ($ty:ty, $value:expr, $len:expr) => {
                for _ in 0..$len {
                    stack_push!($ty, $value);
                }
            };
        }

        macro_rules! stack_push_next_prg {
            ($ty:ty $(as $cast_to:ty)?, $push_ty:ty) => {
                {
                    let value = next_prg!($ty) $(as $cast_to)?;
                    stack_push!($push_ty, value);
                }
            };
        }

        macro_rules! pop {
            ($ptr:expr, $curr_pos:expr, $ty:ty, $err_status:ident) => {
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

        // spec: リターンアドレス以前の領域にアクセス可能
        macro_rules! unsafe_stack_pop {
            ($ty:ty) => {
                pop!(stack_ptr, sp, $ty, StackAccessViolation)
            };

            ($ty:ty, $len:expr) => {
                for _ in 0..$len {
                    unsafe_stack_pop!($ty);
                }
            };
        }

        macro_rules! stack_pop {
            ($ty:ty) => {
                {
                    // note: リターンアドレス以前の値にアクセスしないようチェック
                    if sp < bp + size_of::<usize>() * 2 + size_of::<$ty>() {
                        exit!(StackAccessViolation);
                    }

                    unsafe_stack_pop!($ty)
                }
            };

            ($ty:ty, $len:expr) => {
                for _ in 0..$len {
                    stack_pop!($ty);
                }
            };
        }

        macro_rules! var_table_diff {
            ($ty:ty, $var_i:expr) => {
                {
                    // note: リターンアドレス以前の値にアクセスしないようチェック
                    if sp < bp + size_of::<usize>() * 2 {
                        exit!(StackAccessViolation);
                    }

                    let diff = sp - bp - size_of::<usize>() * 2;

                    // note: スタックポインタ以降の値にアクセスしないようチェック
                    if diff < size_of::<u32>() * $var_i as usize + size_of::<$ty>() {
                        exit!(StackAccessViolation);
                    }

                    diff - $var_i as usize * size_of::<u32>()
                }
            };
        }

        macro_rules! load {
            ($ty:ty, $var_i:expr) => {
                {
                    let diff = var_table_diff!($ty, $var_i);
                    let value = stack_ptr.sub(diff) as *mut $ty;
                    stack_push!($ty, *value);
                }
            };
        }

        macro_rules! store {
            ($ty:ty, $var_i:expr, $value:expr) => {
                let diff = var_table_diff!($ty, $var_i);
                let ptr = stack_ptr.sub(diff) as *mut $ty;
                *ptr = $value
            };
        }

        // spec: リターンアドレス以前の領域にアクセス可能
        macro_rules! top {
            ($ptr:expr, $counter:expr, $ty:ty, $err_status:ident) => {
                {
                    let value_size = size_of::<$ty>();

                    if $counter < value_size {
                        exit!($err_status);
                    }

                    *($ptr as *mut $ty).sub(1)
                }
            };
        }

        macro_rules! unsafe_stack_top {
            ($ty:ty) => {
                top!(stack_ptr, sp, $ty, StackOverflow)
            };
        }

        macro_rules! stack_top {
            ($ty:ty) => {
                {
                    // note: リターンアドレス以前の値にアクセスしないようチェック
                    if sp < bp + size_of::<usize>() * 2 + size_of::<$ty>() {
                        exit!(StackAccessViolation);
                    }

                    unsafe_stack_top!($ty)
                }
            };
        }

        macro_rules! next {
            ($ptr:expr, $curr_pos:expr, $ty:ty, $size:expr, $err_status:ident) => {
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
                next!(inst_ptr, pc, $ty, bytecode_len, BytecodeAccessViolation)
            };
        }

        macro_rules! next_pool {
            ($ty:ty) => {
                next!(pool_ptr, pp, $ty, bytecode_len, BytecodeAccessViolation)
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

                            let zero = if format!("{:0x}", v).len() == 1 { "0" } else { "" };

                            format!("{}{}{:0x} ", div, zero, v)
                        }).collect::<Vec<String>>().join("")
                    } else {
                        "<empty>".to_string()
                    }
                }
            };
        }

        macro_rules! calc {
            ($ty:ty, $f:ident$(, $check_divide_by_zero:expr)?) => {
                {
                    let right_term = stack_pop!($ty);
                    let left_term = stack_pop!($ty);

                    $(
                        if $check_divide_by_zero && right_term == 0 {
                            exit!(DivideByZero);
                        }
                    )?

                    let (value, overflowing) = left_term.$f(right_term);

                    if overflowing {
                        exit!(ArithmeticOverflow);
                    }

                    stack_push!($ty, value);
                }
            };
        }

        macro_rules! goto {
            () => {
                {
                    let offset = next_prg!(i16);
                    let inst_i = pc as isize + offset as isize;

                    if 0 > inst_i {
                        exit!(BytecodeAccessViolation);
                    }

                    jump_prg_to!(inst_i as usize);
                }
            };
        }

        if is_init_succeeded {
            // note: エントリポイント用のコールスタック要素をプッシュ
            println!("{}", "<INVOKE ENTRY POINT>".blue());
            println!();
            // * ベースポインタ
            stack_push!(usize, 0);
            // * リターンアドレス
            stack_push!(usize, bytecode_len - 1);

            'operator: loop {
                // note: 'operator ブロック内での終了処理
                macro_rules! exit {
                    ($status_kind:ident) => {
                        {
                            es = ExitStatus::$status_kind as u32;
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
                    Opcode::Exit => exit!(Success),
                    Opcode::Call => {
                        let code = next_prg!(u8);

                        match code {
                            0x00 => {
                                println!("{}", "[console output]".bright_black());
                                write(1, stack_ptr.sub(size_of::<usize>()), size_of::<usize>() as u32);
                                println!();
                            },
                            _ => exit!(UnknownCallNumber),
                        }
                    },
                    Opcode::Invoke => {
                        let pool_i = next_prg!(usize);
                        jump_pool_to!(pool_i);
                        let start_addr = next_pool!(usize);
                        let var_len = next_pool!(u16) as usize;
                        let arg_len = next_pool!(u8) as usize;

                        if var_len < arg_len || sp < arg_len * size_of::<u32>() {
                            exit!(StackAccessViolation);
                        }

                        // note: 引数値を事前にポップ
                        let mut args = Vec::<u32>::new();

                        for i in 0..arg_len {
                            let new_arg = *((stack_ptr as *mut u32).sub(arg_len - i));
                            args.push(new_arg);
                        }

                        stack_pop!(u32, arg_len);

                        // note: bp をプッシュ & 設定
                        let new_bp = sp;
                        stack_push!(usize, bp);
                        bp = new_bp;

                        // note: リターンアドレスをプッシュ
                        let ret_addr = pc;
                        stack_push!(usize, ret_addr);

                        // note: 引数をプッシュ
                        for each_arg in args {
                            stack_push!(u32, each_arg);
                        }

                        // note: 引数の要素分 (self 参照含む) をスキップ
                        jump_stack_to!(sp + (var_len - arg_len) * size_of::<u32>());

                        // note: 開始アドレスにジャンプ
                        jump_prg_to!(start_addr);

                        println!("{}", format!("[pool index 0x{:0x} / start at 0x{:0x} / return to 0x{:0x} / {} arguments]", pool_i, start_addr, ret_addr, arg_len).bright_green().dimmed());
                        println!();
                    },
                    Opcode::Ret => {
                        if sp < bp || sp - bp < size_of::<usize>() * 2 {
                            exit!(StackAccessViolation);
                        }

                        // note: オペランドスタックと変数テーブルをポップ
                        let pop_size = sp - bp - size_of::<usize>() * 2;
                        unsafe_stack_pop!(u8, pop_size);

                        // note: pc 設定
                        let ret_addr = unsafe_stack_pop!(usize);
                        jump_prg_to!(ret_addr);

                        // note: bp 設定
                        bp = unsafe_stack_pop!(usize);

                        println!("{}", format!("[return to 0x{:0x} / pop {} bytes / return void]", ret_addr, pop_size).bright_green().dimmed());
                        println!();
                    },
                    Opcode::BPush => stack_push_next_prg!(u8 as u32, u32),
                    Opcode::SPush => stack_push_next_prg!(u16 as u32, u32),
                    Opcode::IPush => stack_push_next_prg!(u32, u32),
                    Opcode::LPush => stack_push_next_prg!(u64, u64),
                    Opcode::Dup => {
                        let top_value = stack_top!(u32);
                        stack_push!(u32, top_value);
                    },
                    Opcode::Dup2 => {
                        let top_value = stack_top!(u64);
                        stack_push!(u64, top_value);
                    },
                    Opcode::Pop => {
                        let _ = stack_pop!(u32);
                    },
                    Opcode::Pop2 => {
                        let _ = stack_pop!(u64);
                    },
                    Opcode::Load => {
                        let var_i = next_prg!(u16);
                        load!(u32, var_i);
                    },
                    Opcode::Load2 => {
                        let var_i = next_prg!(u16);
                        load!(u64, var_i);
                    },
                    Opcode::Store => {
                        let var_i = next_prg!(u16);
                        let value = stack_pop!(u32);
                        store!(u32, var_i, value);
                    },
                    Opcode::Store2 => {
                        let var_i = next_prg!(u16);
                        let value = stack_pop!(u64);
                        store!(u64, var_i, value);
                    },
                    Opcode::IAdd => calc!(u32, overflowing_add),
                    Opcode::LAdd => calc!(u64, overflowing_add),
                    Opcode::ISub => calc!(u32, overflowing_sub),
                    Opcode::LSub => calc!(u64, overflowing_sub),
                    Opcode::IMul => calc!(u32, overflowing_mul),
                    Opcode::LMul => calc!(u64, overflowing_mul),
                    Opcode::IDiv => calc!(u32, overflowing_div, true),
                    Opcode::LDiv => calc!(u64, overflowing_div, true),
                    Opcode::IEq => {
                        let value2 = stack_pop!(u32);
                        let value1 = stack_pop!(u32);
                        stack_push!(u32, (value1 == value2) as u32);
                    },
                    Opcode::LEq => {
                        let value2 = stack_pop!(u64);
                        let value1 = stack_pop!(u64);
                        stack_push!(u32, (value1 == value2) as u32);
                    },
                    Opcode::IOrd => {
                        let value2 = stack_pop!(u32);
                        let value1 = stack_pop!(u32);
                        stack_push!(u32, (value1 < value2) as u32);
                    },
                    Opcode::LOrd => {
                        let value2 = stack_pop!(u64);
                        let value1 = stack_pop!(u64);
                        stack_push!(u32, (value1 < value2) as u32);
                    },
                    Opcode::IEqOrd => {
                        let value2 = stack_pop!(u32);
                        let value1 = stack_pop!(u32);
                        stack_push!(u32, (value1 <= value2) as u32);
                    },
                    Opcode::LEqOrd => {
                        let value2 = stack_pop!(u64);
                        let value1 = stack_pop!(u64);
                        stack_push!(u32, (value1 <= value2) as u32);
                    },
                    Opcode::Goto => goto!(),
                    Opcode::If => {
                        let cond = stack_pop!(u32) != 0;

                        if cond {
                            goto!();
                        }

                        let jump_txt = if cond { format!("jump to 0x{:0x}", pc) } else { "no jump".to_string() };
                        println!("{}", format!("[{}]", jump_txt).bright_green().dimmed());
                        println!();
                    },
                    Opcode::Unknown => exit!(UnknownOpcode),
                }
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
