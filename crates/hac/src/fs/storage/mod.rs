use snafu::Snafu;

mod io_storage;

pub trait ReadableStorage {
    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<(), StorageError>;
    fn get_size(&self) -> u64;
}

pub trait Storage: ReadableStorage {
    fn write(&self, offset: u64, buf: &[u8]) -> Result<(), StorageError>;
    fn flush(&self) -> Result<(), StorageError>;
    fn set_size(&self, new_size: u64) -> Result<(), StorageError>;
}

#[derive(Snafu, Debug)]
pub enum StorageError {
    #[snafu(display("IO error in IoStorage: {}", source))]
    Io {
        source: std::io::Error,
        operation: &'static str,
    },
    #[snafu(display("Attempt to write to a read-only storage"))]
    Readonly {},
}
