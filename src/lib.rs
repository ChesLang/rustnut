pub mod bytecode;
pub mod runtime;

use crate::bytecode::*;
use crate::runtime::*;

use rustnutlib::file::*;

pub struct ChesVM {}

impl ChesVM {
    pub fn new() -> ChesVM {
        return ChesVM {};
    }

    pub fn run(&self, chesc_file_path: &str) -> RuntimeResult<()> {
        let file_bytes = match FileMan::read_all_bytes(chesc_file_path) {
            Ok(v) => v,
            Err(e) => return Err(RuntimeError::FileError { err: e }),
        };

        let bytecode = Bytecode::new(file_bytes);
        bytecode.print()?;

        if *HEADER_SIZE > bytecode.len() {
            return Err(RuntimeError::InvalidHeaderSize {});
        }

        if !bytecode.match_bytes(HeaderItem::MagicNumber.get_bytecode_range(), &MAGIC_NUMBER.to_vec()) {
            return Err(RuntimeError::InvalidMagicNumber {});
        }

        unsafe {
            let mut interpreter = Interpreter::launch(bytecode, 1024, 1024)?;
            interpreter.run()?;
        }

        return Ok(());
    }
}
