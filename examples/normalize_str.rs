use eolify::{Normalize, CRLF};

fn main() {
    let text = "Line one\nLine two\rLine three\r\nLine four";
    println!("Before:\n{}", text);

    let normalized = CRLF::normalize_str(text);
    println!("After normalization:\n{}", normalized);
}
