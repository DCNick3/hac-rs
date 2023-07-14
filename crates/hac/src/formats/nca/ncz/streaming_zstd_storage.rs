// this is really bad...

use crate::storage::{ReadableStorage, RoIoStorage, StorageError, StorageIo};
use std::fmt;
use std::io::{BufReader, Seek, SeekFrom};

#[derive(Debug)]
struct FakeSeek<Io, IoReset> {
    io: Io,
    io_reset: IoReset,
    position: u64,
    size: u64,
}

impl<Io: std::io::Read, IoReset: FnMut(Io) -> Io> FakeSeek<Io, IoReset> {
    fn new(io: Io, io_reset: IoReset, size: u64) -> Self {
        Self {
            io,
            io_reset,
            position: 0,
            size,
        }
    }

    fn reset(&mut self) {
        // debug!("Resetting zstd stream position!");
        replace_with::replace_with_or_abort(&mut self.io, &mut self.io_reset);
        self.position = 0;
    }
}

impl<Io: std::io::Read, IoReset: FnMut(Io) -> Io> std::io::Read for FakeSeek<Io, IoReset> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let read = self.io.read(buf)?;
        self.position += read as u64;
        Ok(read)
    }
}

impl<Io: std::io::Read, IoReset: FnMut(Io) -> Io> Seek for FakeSeek<Io, IoReset> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let new_position = match pos {
            SeekFrom::Start(offset) => offset.try_into().unwrap(),
            SeekFrom::End(offset) => self.size as i64 + offset,
            SeekFrom::Current(offset) => self.position as i64 + offset,
        };
        if new_position < 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Attempt to seek before the beginning of the storage",
            ));
        }
        let new_position = new_position as u64;
        if new_position > self.size {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Attempt to seek past the end of the storage",
            ));
        }

        if new_position < self.position {
            self.reset();
        }

        let mut fast_forward_bytes = new_position - self.position;
        let mut buffer = [0u8; 4096];
        while fast_forward_bytes > 0 {
            let read_size = std::cmp::min(fast_forward_bytes, buffer.len() as u64);
            let read = self.io.read(&mut buffer[..read_size as usize])?;
            if read == 0 {
                panic!("Failed to seek to the specified position. Is the size of the underlying storage correct?")
            }
            fast_forward_bytes -= read as u64;
            self.position += read as u64;
        }

        Ok(new_position)
    }

    // TODO: implementing stream_len would be REALLY nice (much better perf than seeking to the end of the stream)
    // I guess we better just special-case the "seek-to-the-end" case
    // but it's unstable =(
}

type RawZstdIo<S> = zstd::Decoder<'static, BufReader<StorageIo<S>>>;
type ZstdIo<S> = FakeSeek<RawZstdIo<S>, fn(RawZstdIo<S>) -> RawZstdIo<S>>;

fn reset_zstd_io<S: ReadableStorage>(io: RawZstdIo<S>) -> RawZstdIo<S> {
    // NOTE: it is BAD to panic here
    // if we panic - the program will be aborted (because we use replace_with_or_abort)
    let mut io = io.finish();
    io.seek(SeekFrom::Start(0))
        .expect("Failed to seek to the beginning of the underlying Zstd stream");
    zstd::Decoder::with_buffer(io).expect("Failed to create a new Zstd stream")
}

/// This storage decompresses the underlying storage using Zstd.
///
/// It is VERY inefficient when you try to read it non-sequentially. (it basically has to re-start the decompression from the beginning)
pub struct StreamingZstdStorage<S: ReadableStorage> {
    storage: RoIoStorage<ZstdIo<S>>,
}

impl<S: ReadableStorage> fmt::Debug for StreamingZstdStorage<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("StreamingZstdStorage").finish()
    }
}

impl<S: ReadableStorage> StreamingZstdStorage<S> {
    pub fn new(storage: S, uncompressed_size: u64) -> Result<Self, StorageError> {
        let io = zstd::Decoder::with_buffer(BufReader::new(StorageIo::new(storage)))
            .expect("Failed to create a new Zstd stream");
        let io = FakeSeek::new(
            io,
            reset_zstd_io as _, /* this "as" is unfortunate =( */
            uncompressed_size,
        );
        let storage = RoIoStorage::new_with_size(io, uncompressed_size);

        Ok(Self { storage })
    }
}

impl<S: ReadableStorage> ReadableStorage for StreamingZstdStorage<S> {
    fn read(&self, offset: u64, buf: &mut [u8]) -> Result<(), StorageError> {
        self.storage.read(offset, buf)
    }

    fn get_size(&self) -> u64 {
        self.storage.get_size()
    }
}
