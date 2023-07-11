use binrw::{BinRead, BinWrite};
use binrw::{BinResult, Endian};
use std::io::{Read, Seek, Write};

pub fn read_bool<R: Read>(reader: &mut R, _endian: Endian, _args: ()) -> BinResult<bool> {
    let mut buf = [0u8; 1];
    reader.read_exact(&mut buf)?;
    Ok(buf[0] != 0)
}

pub fn write_bool<W: Write>(
    value: &bool,
    writer: &mut W,
    _endian: Endian,
    _args: (),
) -> BinResult<()> {
    writer.write_all(&[u8::from(*value)])?;
    Ok(())
}

pub fn read_u40<R: Read + Seek>(reader: &mut R, endian: Endian, _args: ()) -> BinResult<u64> {
    assert_eq!(endian, Endian::Little);
    let low = u32::read_le(reader)?;
    let high = u8::read_le(reader)?;
    Ok((high as u64) << 32 | (low as u64))
}

pub fn write_u40<W: Write + Seek>(
    value: &u64,
    writer: &mut W,
    endian: Endian,
    _args: (),
) -> BinResult<()> {
    assert_eq!(endian, Endian::Little);
    assert!(value <= &0xFF_FFFF_FFFF);
    let low = (value & 0xFFFF_FFFF) as u32;
    let high = (value >> 32) as u8;
    low.write_le(writer)?;
    high.write_le(writer)?;
    Ok(())
}

pub fn read_u48<R: Read + Seek>(reader: &mut R, endian: Endian, _args: ()) -> BinResult<u64> {
    assert_eq!(endian, Endian::Little);
    let low = u32::read_le(reader)?;
    let high = u16::read_le(reader)?;
    Ok((high as u64) << 32 | (low as u64))
}

pub fn write_u48<W: Write + Seek>(
    value: &u64,
    writer: &mut W,
    endian: Endian,
    _args: (),
) -> BinResult<()> {
    assert_eq!(endian, Endian::Little);
    assert!(value <= &0xFFFF_FFFF_FFFF);
    let low = (value & 0xFFFF_FFFF) as u32;
    let high = (value >> 32) as u16;
    low.write_le(writer)?;
    high.write_le(writer)?;
    Ok(())
}

pub fn read_u48_rev<R: Read + Seek>(reader: &mut R, endian: Endian, _args: ()) -> BinResult<u64> {
    assert_eq!(endian, Endian::Little);
    let high = u16::read_le(reader)?;
    let low = u32::read_le(reader)?;
    Ok((high as u64) << 32 | (low as u64))
}

pub fn write_u48_rev<W: Write + Seek>(
    value: &u64,
    writer: &mut W,
    endian: Endian,
    _args: (),
) -> BinResult<()> {
    assert_eq!(endian, Endian::Little);
    let high = (value >> 32) as u16;
    let low = (value & 0xFFFF_FFFF) as u32;
    low.write_le(writer)?;
    high.write_le(writer)?;
    Ok(())
}
