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
    InvalidTermination {},
    StackOverflow {},
    UnknownOpcode {},
}

impl ConsoleLogger for RuntimeError {
    fn get_log(&self) -> ConsoleLog {
        return match self {
            RuntimeError::Unknown {} => log!(Error, "unknown error"),
            RuntimeError::FileError { err } => err.get_log(),
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

pub struct Interpreter {
    bytecode: Box<Vec<u8>>,
    program_counter: usize,
    stack: *mut u64,
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

        let interpreter = Interpreter {
            bytecode: bytecode.into_vec(),
            program_counter: 127,
            stack: malloc(size_of::<u8>() * stack_addition_size * 1000) as *mut u64,
            stack_counter: 0,
            max_stack_size: max_stack_size,
            stack_addition_size: stack_addition_size,
        };

        return Ok(interpreter);
    }

    // arg: max_stack_size: KB 単位
    pub unsafe fn run(&mut self) -> RuntimeResult<()> {
        // println!("add {}", *stack.add(1));
        // *stack = 0xabcd;
        // println!("value {:0x}", *stack as u8);

        loop {
            // 終了命令がないまま EOF に到達した場合
            if self.program_counter >= self.bytecode.len() {
                return Err(RuntimeError::InvalidTermination {});
            }

            let opcode = self.next_byte();
            println!("opcode 0x{:0x}", opcode);
            match opcode {
                // RET
                0x00 => (),
                // EXIT
                0x01 => break,
                // PUSH_8
                0x02 => {
                    let v = self.next_byte() as u64;
                    println!("\tpush {:?}", std::slice::from_raw_parts(self.stack, self.stack_counter));
                    self.push(v);
                    println!("\tpush {:?}", std::slice::from_raw_parts(self.stack, self.stack_counter));
                },
                // POP
                0x03 => {
                    let _ = self.pop();
                    ()
                },
                // IADD_8
                0x04 => {
                    println!("\tpop {:?}", std::slice::from_raw_parts(self.stack, self.stack_counter));
                    let v1 = self.pop();
                    println!("\t{}", v1);
                    println!("\tpop {:?}", std::slice::from_raw_parts(self.stack, self.stack_counter));
                    let v2 = self.pop();
                    println!("\t{}", v2);
                    println!("\tpop {:?}", std::slice::from_raw_parts(self.stack, self.stack_counter));
                    println!("\t{} + {} = {}", v1, v2, v1 + v2);
                    self.push(v1 + v2);
                },
                _ => return Err(RuntimeError::UnknownOpcode {})
            }
        }

        println!("");
        println!("- stack -");
        println!("");
        println!("{:?}", std::slice::from_raw_parts(self.stack, self.stack_counter));
        println!("{:?}", std::slice::from_raw_parts(self.stack, 5));

        self.terminate();
        return Ok(());
    }

    // note: メモリリークを防ぐため終了時は必ず呼び出すこと
    unsafe fn terminate(&mut self) {
        free(self.stack as *mut c_void);
    }

    #[inline(always)]
    unsafe fn push(&mut self, value: u64) {
        if self.stack_counter + 1 >= self.max_stack_size {
            panic!("stack access violation (stack overflow)");
        }

        println!("\tpush value {} {}", value, *self.stack);

        if self.stack_counter == 0 {
            *self.stack = value;
        } else {
            let a = self.stack.add(1);
            *a = value;
        }

        self.stack_counter += 1;
    }

    #[inline(always)]
    unsafe fn pop(&mut self) -> u64 {
        if self.stack_counter == 0 {
            panic!("stack access violation");
        }

        let pop_value = *self.stack;
        self.stack_counter -= 1;
        let pop_value = self.stack.offset(-1);
        return *pop_value;
    }

    #[inline(always)]
    fn next_byte(&mut self) -> u8 {
        self.program_counter += 1;
        return self.bytecode[self.program_counter];
    }
}
