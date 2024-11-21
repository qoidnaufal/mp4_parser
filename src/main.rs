use parse_mp4::Mp4;
use std::io;

fn main() -> io::Result<()> {
    let bytes = std::fs::read("../vid1.mp4")?;
    let len = bytes.len();
    let reader = io::Cursor::new(bytes);

    let parsed_mp4 = Mp4::read(reader, len as _)?;
    let traks = parsed_mp4.moov.traks;

    for trak in traks {
        println!("{:?}", trak.mdia.minf.stbl)
    }

    Ok(())
}
