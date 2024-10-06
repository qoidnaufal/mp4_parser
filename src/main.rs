use std::fmt::Display;
use std::fs::File;
use std::io::{self, BufReader, Read, Seek};
use std::path::Path;

#[derive(Debug)]
enum FourCC {
    FTYP,
    MOOV,
    UUID,
    FREE,
    MDAT,
    DAT,
    #[allow(dead_code)]
    OTHER(String),
}

impl From<&[u8; 4]> for FourCC {
    fn from(value: &[u8; 4]) -> Self {
        match value {
            b"ftyp" => Self::FTYP,
            b"moov" => Self::MOOV,
            b"uuid" => Self::UUID,
            b"free" => Self::FREE,
            b"mdat" => Self::MDAT,
            b"dat\0" => Self::DAT,
            v => {
                let other_name = String::from_utf8_lossy(v).to_string();
                Self::OTHER(other_name)
            }
        }
    }
}

#[derive(Debug)]
struct Atom {
    #[allow(dead_code)]
    name: FourCC,
    size: u32,
    #[allow(dead_code)]
    offset: u32,
    #[allow(dead_code)]
    data: usize,
}

#[allow(dead_code)]
fn get_data<R>(reader: &mut R, size: u32, offset: u32) -> io::Result<usize>
where
    R: Read + Seek,
{
    reader.seek(io::SeekFrom::Start((offset + 8) as _))?;
    let mut data_buf = vec![0; (size - 8) as _];
    reader.read_exact(&mut data_buf)?;
    Ok(data_buf.len())
}

fn get_extended_data<R>(reader: &mut R, offset: u32) -> io::Result<usize>
where
    R: Read + Seek,
{
    reader.seek(io::SeekFrom::Start(offset as _))?;
    let mut data_buf = Vec::new();
    reader.read_to_end(&mut data_buf)?;
    data_buf.shrink_to_fit();
    Ok(data_buf.len())
}

fn get_extended_size<R>(reader: &mut R, offset: u32) -> io::Result<u32>
where
    R: Read + Seek,
{
    reader.seek(io::SeekFrom::Start((offset + 8) as _))?;
    let mut size_buf = [0; 4];

    reader.read_exact(&mut size_buf)?;
    let size = u32::from_be_bytes(size_buf);

    Ok(size)
}

fn read_box<R>(reader: &mut R, offset: u32) -> io::Result<Atom>
where
    R: Read + Seek,
{
    let mut size_buf = [0; 4];
    let mut type_buf = [0; 4];

    reader.read_exact(&mut size_buf)?;
    reader.read_exact(&mut type_buf)?;

    let size = u32::from_be_bytes(size_buf);
    let name = FourCC::from(&type_buf);

    // size == 1 -> check the following 4 bytes
    let size = if size == 1 {
        get_extended_size(reader, offset)?
    } else {
        size
    };

    // size == 0 -> the atom extends to the end
    let data = if size > 0 {
        get_data(reader, size, offset)?
    } else {
        get_extended_data(reader, offset)?
    };

    Ok(Atom {
        name,
        size,
        offset,
        data,
    })
}

fn parse_mp4<P>(file_path: P) -> io::Result<()>
where
    P: AsRef<Path> + Display,
{
    let file = File::open(&file_path)?;
    let len = file.metadata()?.len();
    println!("file size: {len}");
    let mut reader = BufReader::new(file);

    let mut offset = 0;

    while let Ok(atom) = read_box(&mut reader, offset) {
        println!("{:?}", atom);

        offset += atom.size;

        // reader.seek(io::SeekFrom::Current(atom.size as i64 - 8))?;
    }

    Ok(())
}

// fn extract_bytes<P: AsRef<Path> + Display>(file_path: P) -> io::Result<()> {
//     let file = File::open(&file_path)?;
//     let len = file.metadata()?.len() as usize;

//     let mut reader = BufReader::new(file);
//     let mut buf = Vec::with_capacity(len);

//     reader.read_to_end(&mut buf)?;

//     let mut iter = buf.chunks(8);
//     let mut offset = 0;

//     while let Some(data) = iter.next() {
//         if offset >= 24 {
//             println!("{data:02X?} {}", data_to_string(data));
//         }
//         offset += 8;

//         if offset > 1314 {
//             break;
//         }
//     }

//     Ok(())
// }

// fn data_to_string(data: &[u8]) -> String {
//     data.iter()
//         .map(|n| {
//             if *n == 0 || !n.is_ascii_alphanumeric() {
//                 '.'
//             } else {
//                 *n as _
//             }
//         })
//         .collect()
// }

fn main() -> io::Result<()> {
    let file_path = "../vid1.mp4";
    parse_mp4(file_path)
}
