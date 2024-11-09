use std::io;

fn main() -> io::Result<()> {
    let bytes = std::fs::read("../vid1.mp4")?;
    let len = bytes.len();
    let reader = io::Cursor::new(bytes);

    read(reader, len as _)?;

    Ok(())
}
