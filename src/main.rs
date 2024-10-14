#![allow(dead_code)]

use std::fmt::Display;
use std::fs::File;
use std::io::{self, BufReader, Read, Seek};
use std::path::Path;

#[derive(Debug)]
enum AtomName {
    FTYP,
    MOOV,
    UUID,
    FREE,
    MDAT,
    OTHER(String),
}

impl From<&[u8; 4]> for AtomName {
    fn from(value: &[u8; 4]) -> Self {
        match value {
            b"ftyp" => Self::FTYP,
            b"moov" => Self::MOOV,
            b"uuid" => Self::UUID,
            b"free" => Self::FREE,
            b"mdat" => Self::MDAT,
            other => {
                let other_name = String::from_utf8_lossy(other).to_string();
                Self::OTHER(other_name)
            }
        }
    }
}

#[derive(Debug)]
enum AtomData {
    Mdat(usize),
    Moov(Vec<u8>),
    Other(usize),
}

#[derive(Debug)]
struct Atom {
    // name: AtomName
    name: String,
    size: u32,
    ext_size: Option<u64>,
    offset: u32,
    data: AtomData,
}

fn get_data<R>(reader: &mut R, name: &str, size: u32, offset: u32) -> io::Result<AtomData>
where
    R: Read + Seek,
{
    reader.seek(io::SeekFrom::Start((offset + 8) as _))?;

    let data = match name {
        "moov" => {
            let mut data_buf = vec![0; (size - 8) as _];
            reader.read_exact(&mut data_buf)?;
            AtomData::Moov(data_buf)
        }
        "mdat" => {
            let mut data_buf = vec![0; (size - 8) as _];
            reader.read_exact(&mut data_buf)?;
            AtomData::Mdat(data_buf.len())
        }
        _ => {
            let mut data_buf = vec![0; (size - 8) as _];
            reader.read_exact(&mut data_buf)?;
            AtomData::Other(data_buf.len())
        }
    };

    Ok(data)
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

fn get_extended_size<R>(reader: &mut R, offset: u32) -> io::Result<u64>
where
    R: Read + Seek,
{
    reader.seek(io::SeekFrom::Start((offset + 8) as _))?;

    let mut size_buf = [0; 8];
    reader.read_exact(&mut size_buf)?;

    Ok(u64::from_be_bytes(size_buf))
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
    // let name = AtomName::from(&type_buf);
    let name = String::from_utf8_lossy(&type_buf).to_string();

    // size == 1 -> check the following 8 bytes (u64) data to get the actual size
    let (size, ext_size) = if size == 1 {
        (1, Some(get_extended_size(reader, offset)?))
    } else {
        (size, None)
    };

    let data = if ext_size.is_none() {
        get_data(reader, &name, size, offset)?
    } else {
        let data = get_extended_data(reader, offset)?;
        assert_eq!(ext_size.unwrap(), data as _);
        AtomData::Mdat(data)
    };

    Ok(Atom {
        name,
        size,
        ext_size,
        offset,
        data,
    })
}

fn parse_mp4<P>(file_path: P) -> io::Result<()>
where
    P: AsRef<Path> + Display,
{
    let file = File::open(&file_path)?;

    let mut reader = BufReader::new(file);
    let mut offset = 0;

    while let Ok(atom) = read_box(&mut reader, offset) {
        println!("{:?}", atom);
        offset += atom.size;
    }

    Ok(())
}

fn main() -> io::Result<()> {
    let file_path = "../vid1.mp4";
    parse_mp4(file_path)
    // print_bytes_layout(file_path)
}

fn print_bytes_layout<P: AsRef<Path> + Display>(file_path: P) -> io::Result<()> {
    let file = File::open(&file_path)?;
    let len = file.metadata()?.len() as usize;

    let mut reader = BufReader::new(file);
    let mut buf = Vec::with_capacity(len);

    reader.read_to_end(&mut buf)?;

    let mut iter = buf.chunks(8);
    let mut offset = 0;

    while let Some(data) = iter.next() {
        if offset >= 24 {
            println!("{data:02X?} {}", data_to_string(data));
        }
        offset += 8;

        if offset > 1314 {
            break;
        }
    }

    Ok(())
}

fn data_to_string(data: &[u8]) -> String {
    data.iter()
        .map(|n| {
            if *n == 0 || !n.is_ascii_alphanumeric() {
                '.'
            } else {
                *n as _
            }
        })
        .collect()
}
