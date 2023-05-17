use std::io::{Write, Read};
use log::{trace, debug};

pub const CONFIG: bincode::config::Configuration = bincode::config::standard();

#[derive(Debug)]
enum BlockKind {
    Partial,
    Final
}

#[derive(Debug)]
struct Header {
    length: u32,
    kind: BlockKind,
}

impl Header {
    pub fn to_bytes(&self) -> [u8; 5] {
        let mut buf = [0u8; 5];

        buf[4] = match self.kind {
            BlockKind::Partial => 0,
            BlockKind::Final => 1,
        };

        buf[..4].copy_from_slice(&self.length.to_le_bytes());

        buf
    }

    pub fn from_bytes(buf: [u8; 5]) -> Self {
        let kind = match buf[4] {
            0 => BlockKind::Partial,
            1 => BlockKind::Final,
            kind => panic!("Unknown BlockKind is received {kind}"),
        };

        Header { length: u32::from_le_bytes(buf[..4].try_into().unwrap()), kind }
    }
}

pub struct Compressor<W> {
    dest: W,
    buffer: Vec<u8>,
    comp: flate2::Compress,
}

impl<W> Compressor<W> {
    pub fn new(dest: W, level: u32) -> Self {
        Compressor {
            dest,
            buffer: vec![0u8; 1024 * 8],
            comp: flate2::Compress::new(flate2::Compression::new(level), true),
        }
    }

    pub fn total_in(&self) -> u64 {
        self.comp.total_in()
    }

    pub fn total_out(&self) -> u64 {
        self.comp.total_out()
    }
}

pub struct Decompressor<R> {
    source: R,
    decomp: flate2::Decompress,
    buffer: Vec<u8>,
    start: usize,
    end: usize,
    header: Header,
}

impl<R> Decompressor<R> {
    pub fn new(source: R) -> Self {
        Decompressor {
            source,
            decomp: flate2::Decompress::new(true),
            buffer: vec![0u8; 1024 * 8],
            start: 0,
            end: 0,
            header: Header { length: 0, kind: BlockKind::Partial },
        }
    }

    pub fn total_in(&self) -> u64 {
        self.decomp.total_in()
    }

    pub fn total_out(&self) -> u64 {
        self.decomp.total_out()
    }
}

impl<R: Read> Decompressor<R> {
    pub fn finish(&mut self) -> std::io::Result<()> {
        trace!("total read {}, {}", self.decomp.total_out(), self.decomp.total_in());
        trace!("Last header in finish {:?}", self.header);

        loop {
            if self.header.length == 0 {
                if let BlockKind::Final = self.header.kind {
                    break;
                }

                let mut bytes = [0u8; 5];
                self.source.read_exact(&mut bytes).unwrap();
                self.header = Header::from_bytes(bytes);
                trace!("New header in finish {:?}", self.header);
            }

            let max_len = std::cmp::min(self.buffer.len(), self.header.length as usize);
            if max_len > 0 {
                self.header.length -= self.source.read(&mut self.buffer[..max_len]).unwrap() as u32;
            }
        }

        return Ok(());
    }
}

impl<R> Read for Decompressor<R> where R: Read {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.end - self.start == 0 {
            self.start = 0;
            self.end = 0;

            if self.header.length > 0 {
                let max_len = std::cmp::min(self.buffer.len(), self.header.length as usize);
                self.end = self.source.read(&mut self.buffer[..max_len]).unwrap();
                trace!("read remaining bytes {}, {}", self.end, self.header.length);
                self.header.length -= self.end as u32;
            } else if let BlockKind::Partial = self.header.kind {
                let mut bytes = [0u8; 5];
                self.source.read_exact(&mut bytes).unwrap();
                self.header = Header::from_bytes(bytes);
                trace!("received new header {:?}", self.header);
                return self.read(buf);
            } else {
                debug!("Neither there is remaining bytes to read nor the header was partial");
            }
        }
        let (before_in, before_out) = (self.decomp.total_in(), self.decomp.total_out());

        match self.decomp.decompress(&self.buffer[self.start..self.end], buf, flate2::FlushDecompress::None).unwrap() {
            flate2::Status::Ok => { },
            flate2::Status::StreamEnd => trace!("Stream is ended"),
            flate2::Status::BufError => panic!("BufError should not be received"),
        }

        let (after_in, after_out) = (self.decomp.total_in(), self.decomp.total_out());

        self.start += <u64 as TryInto<usize>>::try_into(after_in - before_in).unwrap();

        let produced_bytes = <u64 as TryInto<usize>>::try_into(after_out - before_out).unwrap();
        if produced_bytes == 0 {
            return self.read(buf);
        }

        Ok(produced_bytes)
    }
}

impl<W> Write for Compressor<W> where W: Write {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let (before_in, before_out) = (self.comp.total_in(), self.comp.total_out());

        match self.comp.compress(buf, &mut self.buffer, flate2::FlushCompress::None).unwrap() {
            flate2::Status::Ok => { },
            status => panic!("Failed to compress {status:?}"),
        }

        let (after_in, after_out) = (self.comp.total_in(), self.comp.total_out());

        let should_written: usize = (after_out - before_out).try_into().unwrap();
        if should_written > 0 {
            let header = Header { kind: BlockKind::Partial, length: (after_out - before_out) as u32 };
            trace!("Sending new header {header:?}");
            self.dest.write_all(&header.to_bytes()).unwrap();
            self.dest.write_all(&self.buffer[..should_written]).unwrap();
        }

        if after_in - before_in == 0 {
            return self.write(buf);
        }

        Ok((after_in - before_in).try_into().unwrap())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        trace!("flushing, {}, {}", self.comp.total_in(), self.comp.total_out());
        loop {
            let before_out = self.comp.total_out();
            match self.comp.compress(&[], &mut self.buffer, flate2::FlushCompress::Finish).unwrap() {
                flate2::Status::BufError => panic!("Failed to flush compress due to BufError"),
                status => {
                    let compressed: usize = (self.comp.total_out() - before_out).try_into().unwrap();

                    if compressed == 0 {
                        break;
                    }

                    let kind = match status {
                        flate2::Status::StreamEnd => BlockKind::Final,
                        _ => BlockKind::Partial,
                    };

                    let header = Header { kind, length: compressed as u32 };
                    trace!("Sending new header in flush {header:?}");
                    self.dest.write_all(&header.to_bytes()).unwrap();
                    self.dest.write_all(&self.buffer[..compressed]).unwrap();
                },
            }
        }

        self.dest.flush().unwrap();
        trace!("flushed, {}, {}", self.comp.total_in(), self.comp.total_out());
        Ok(())
    }
}
