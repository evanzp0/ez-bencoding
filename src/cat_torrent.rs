use std::fs;
use std::io;
use std::env;
use std::io::Read;

use ez_bencoding::BdecodeNode;

fn main() -> io::Result<()> {
    // 获取命令行参数
    let args: Vec<String> = env::args().collect();

    // 检查是否提供了文件路径作为参数
    if args.len() < 2 {
        eprintln!("Usage: {:?} <file_path>", args);
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "No file path provided"));
    }

    // 获取文件路径
    let file_path = &args[1];
    let mut file = fs::File::open(file_path).expect("Failed to open file");

    // 读取文件内容到 Vec<u8>
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect("Failed to read file");

    let root_node = BdecodeNode::parse_buffer(buffer.into()).unwrap();
    println!("{}", &root_node.to_json_pretty());

    Ok(())
}
