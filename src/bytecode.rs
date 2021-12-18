use crate::runtime::*;

pub const HEADER_SIZE: &'static usize = &128;

pub const CURRENT_CHES_VERSION: &'static (usize, usize, usize) = &(1, 0, 0);
pub const MAGIC_NUMBER: &'static [u8; 8] = &[0x43u8, 0x48u8, 0x45u8, 0x53u8, 0x43u8, 0x43u8, 0x42u8, 0x43u8];

pub struct BytecodeRange {
    pub begin: usize,
    pub len: usize,
}

impl BytecodeRange {
    pub fn new(begin: usize, len: usize) -> BytecodeRange {
        return BytecodeRange {
            begin: begin,
            len: len,
        }
    }
}

pub struct Bytecode {
    bytes: Box<Vec<u8>>,
}

impl Bytecode {
    pub fn new(bytes: Vec<u8>) -> Bytecode {
        return Bytecode {
            bytes: Box::new(bytes),
        };
    }

    pub fn print(&self) -> RuntimeResult<()> {
        println!("- Ches Bytecode -");
        println!();
        println!("MAGIC NUMBER\t{}", Bytecode::bytes_to_string(&self.get_bytes(HeaderItem::MagicNumber.get_bytecode_range())?));
        println!("CODE NAME\t{}", Bytecode::bytes_to_string(&self.get_bytes(HeaderItem::CodeName.get_bytecode_range())?));
        println!("CHES VERSION\t{}", Bytecode::bytes_to_string(&self.get_bytes(HeaderItem::ChesVersion.get_bytecode_range())?));
        println!();
        println!("{}", Bytecode::bytes_to_string(&*self.bytes));
        println!();

        return Ok(());
    }

    pub fn len(&self) -> usize {
        return self.bytes.len();
    }

    pub fn into_vec(self) -> Box<Vec<u8>> {
        return self.bytes;
    }

    pub fn bytes_to_string(bytes: &Vec<u8>) -> String {
        return bytes.iter().map(|v| {
            let s = format!("{:0x}", v);
            if s.len() == 1 { format!("0{}", s) } else { s }
        }).collect::<Vec<String>>().join(" ");
    }

    pub fn get_bytes(&self, range: BytecodeRange) -> RuntimeResult<Vec<u8>> {
        return if range.begin + range.len > self.bytes.len() {
            Err(RuntimeError::IndexOutOfBytecodeRange {})
        } else {
            Ok(self.bytes[range.begin..range.begin + range.len].to_vec().clone())
        };
    }

    pub fn match_bytes(&self, range: BytecodeRange, pattern: &Vec<u8>) -> bool {
        return match self.get_bytes(range) {
            Ok(v) => *pattern == v,
            Err(_) => false,
        }
    }
}

pub enum HeaderItem {
    MagicNumber,
    CodeName,
    ChesVersion,
}

impl HeaderItem {
    pub fn get_bytecode_range(&self) -> BytecodeRange {
        let (begin, len) = match self {
            HeaderItem::MagicNumber => (0, 8),
            HeaderItem::CodeName => (8, 8),
            HeaderItem::ChesVersion => (16, 3),
        };

        return BytecodeRange::new(begin, len);
    }
}
