pub mod bytecode;
pub mod runtime;

use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use cc::*;

use crate::bytecode::*;
use crate::runtime::*;

use rustnutlib::file::*;

pub struct ChesVM {}

impl ChesVM {
    pub fn new() -> ChesVM {
        return ChesVM {};
    }

    pub fn build_assembly() -> std::io::Result<()> {
        // このクレートのビルドディレクトリです
        let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

        // ライブラリサーチパスを追加します
        println!("cargo:rustc-link-search={}", out_dir.display());

        // `link.x`をビルドディレクトリに置きます
        File::create(out_dir.join("link.x"))?.write_all(include_bytes!("asm/build/link.x"))?;

        // `asm.s`ファイルをアセンブルします
        Build::new().file(r"C:\Users\Garnet3106\Desktop\Media\Docs\Repos\ChesLang\rustnut\src\asm\test.s").out_dir(r"C:\Users\Garnet3106\Desktop\Media\Docs\Repos\ChesLang\rustnut\src\asm\build").opt_level(1).host("x86_64-pc-windows-gnu").target("x86_64-pc-windows-gnu").compile("asm");
        return Ok(());
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
            Interpreter::run(&bytecode, 10, 2)?;
        }

        return Ok(());
    }
}
