use eolify::{TokioExt, CRLF};
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufReader, BufWriter};

#[tokio::main(flavor = "current_thread")]
async fn main() -> tokio::io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <input> <output>", args[0]);
        std::process::exit(1);
    }
    let input_path = &args[1];
    let output_path = &args[2];

    let infile = File::open(input_path).await?;
    let mut reader = BufReader::new(infile);

    let outfile = File::create(output_path).await?;
    let mut writer = CRLF::wrap_async_writer(BufWriter::new(outfile));

    tokio::io::copy(&mut reader, &mut writer).await?;
    writer.flush().await?;
    Ok(())
}
