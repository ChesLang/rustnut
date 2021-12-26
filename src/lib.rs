pub mod bytecode;
pub mod runtime;

use crate::runtime::*;

use rustnutlib::file::*;

pub struct ChesVM {}

impl ChesVM {
    pub fn new() -> ChesVM {
        return ChesVM {};
    }

    pub fn run(&self, chesc_file_path: &str) -> FileResult<ExitStatus> {
        let file_bytes = match FileMan::read_all_bytes(chesc_file_path) {
            Ok(v) => v,
            Err(e) => return Err(e),
        };

        unsafe {
            return Ok(Interpreter::launch(file_bytes));
        }
    }
}
