use binrw::{BinResult, ReadOptions, WriteOptions};
use std::io::{Read, Write};

pub fn read_bool<R: Read>(reader: &mut R, _options: &ReadOptions, _args: ()) -> BinResult<bool> {
    let mut buf = [0u8; 1];
    reader.read_exact(&mut buf)?;
    Ok(buf[0] != 0)
}

pub fn write_bool<W: Write>(
    value: &bool,
    writer: &mut W,
    _options: &WriteOptions,
    _args: (),
) -> BinResult<()> {
    writer.write_all(&[u8::from(*value)])?;
    Ok(())
}
