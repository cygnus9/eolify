use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};

use eolify::{IoExt, CRLF};

fn main() -> std::io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <input> <output>", args[0]);
        std::process::exit(1);
    }
    let input_path = &args[1];
    let output_path = &args[2];

    let infile = File::open(input_path)?;
    let mut reader = CRLF::wrap_reader(infile);

    let outfile = File::create(output_path)?;
    let mut writer = BufWriter::new(outfile);

    std::io::copy(&mut reader, &mut writer)?;
    writer.flush()?;
    Ok(())
}
