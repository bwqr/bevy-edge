use std::io::{Write, Read};
use std::time::{Duration, Instant};
use log::trace;

pub const CONFIG: bincode::config::Configuration = bincode::config::standard();

#[derive(Debug)]
struct Header {
    length: usize,
    finished: bool,
}

impl Header {
    pub fn to_bytes(&self) -> [u8; 5] {
        let mut buf = [0u8; 5];

        buf[4] = if self.finished { 1 } else { 0 };

        buf[..4].copy_from_slice(&(self.length as u32).to_le_bytes());

        buf
    }

    pub fn from_bytes(buf: [u8; 5]) -> Self {
        let finished = match buf[4] {
            0 => false,
            1 => true,
            kind => panic!("Invalid value for bool is received {kind}"),
        };

        Header { length: u32::from_le_bytes(buf[..4].try_into().unwrap()) as usize, finished }
    }
}

pub struct Compressor<W> {
    dest: W,
    buffer: Vec<u8>,
    comp: flate2::Compress,
    duration: Duration,
}

impl<W> Compressor<W> {
    pub fn new(dest: W, level: u32) -> Self {
        Compressor {
            dest,
            buffer: vec![0u8; 1024 * 8],
            comp: flate2::Compress::new(flate2::Compression::new(level), true),
            duration: Duration::new(0, 0),
        }
    }

    pub fn total_in(&self) -> u64 {
        self.comp.total_in()
    }

    pub fn total_out(&self) -> u64 {
        self.comp.total_out()
    }

    pub fn elapsed(&self) -> u32 {
        self.duration.as_micros().try_into().unwrap()
    }
}

pub struct Decompressor<R> {
    source: R,
    decomp: flate2::Decompress,
    header: Header,
    buffer: Vec<u8>,
    duration: Duration,
    start: usize,
    end: usize,
}

impl<R> Decompressor<R> {
    pub fn new(source: R) -> Self {
        Decompressor {
            source,
            decomp: flate2::Decompress::new(true),
            header: Header { length: 0, finished: false },
            buffer: vec![0u8; 1024 * 8],
            duration: Duration::new(0, 0),
            start: 0,
            end: 0,
        }
    }

    pub fn total_in(&self) -> u64 {
        self.decomp.total_in()
    }

    pub fn total_out(&self) -> u64 {
        self.decomp.total_out()
    }

    pub fn elapsed(&self) -> u32 {
        self.duration.as_micros().try_into().unwrap()
    }
}

impl<R: Read> Decompressor<R> {
    pub fn finish(&mut self) -> std::io::Result<()> {
        trace!("total read {}, {}", self.decomp.total_out(), self.decomp.total_in());
        trace!("Last header in finish {:?}", self.header);

        loop {
            if self.header.length == 0 {
                if self.header.finished {
                    break;
                }

                self.read_header();

                trace!("New header in finish {:?}", self.header);
            }

            let max_len = std::cmp::min(self.buffer.len(), self.header.length);
            if max_len > 0 {
                self.header.length -= self.source.read(&mut self.buffer[..max_len]).unwrap();
            }
        }

        return Ok(());
    }

    fn read_header(&mut self) {
        let mut bytes = [0u8; 5];
        self.source.read_exact(&mut bytes).unwrap();
        self.header = Header::from_bytes(bytes);
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
                self.header.length -= self.end;
            } else if !self.header.finished {
                self.read_header();
                trace!("received new header {:?}", self.header);
                return self.read(buf);
            }
        }
        let (before_in, before_out) = (self.decomp.total_in(), self.decomp.total_out());

        let instant = Instant::now();
        let status = self.decomp.decompress(&self.buffer[self.start..self.end], buf, flate2::FlushDecompress::None).unwrap();
        self.duration = self.duration.saturating_add(instant.elapsed());

        match status {
            flate2::Status::Ok => { },
            flate2::Status::StreamEnd => trace!("Stream is ended"),
            flate2::Status::BufError => panic!("BufError should not be received"),
        }

        let (after_in, after_out) = (self.decomp.total_in(), self.decomp.total_out());
        self.start += <u64 as TryInto<usize>>::try_into(after_in - before_in).unwrap();

        let produced_bytes = (after_out - before_out).try_into().unwrap();
        if produced_bytes == 0 {
            return self.read(buf);
        }

        Ok(produced_bytes)
    }
}

impl<W> Write for Compressor<W> where W: Write {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let (before_in, before_out) = (self.comp.total_in(), self.comp.total_out());

        let instant = Instant::now();
        let status = self.comp.compress(buf, &mut self.buffer, flate2::FlushCompress::None).unwrap();
        self.duration = self.duration.saturating_add(instant.elapsed());

        match status {
            flate2::Status::Ok => { },
            status => panic!("Failed to compress {status:?}"),
        }

        let (after_in, after_out) = (self.comp.total_in(), self.comp.total_out());

        let produced_bytes: usize = (after_out - before_out).try_into().unwrap();
        if produced_bytes > 0 {
            let header = Header { length: (after_out - before_out).try_into().unwrap(), finished: false };
            trace!("Sending new header {header:?}");
            self.dest.write_all(&header.to_bytes()).unwrap();
            self.dest.write_all(&self.buffer[..produced_bytes]).unwrap();
        }

        let consumed_bytes = (after_in - before_in).try_into().unwrap();
        if  consumed_bytes == 0 {
            return self.write(buf);
        }

        Ok(consumed_bytes)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        trace!("flushing, {}, {}", self.comp.total_in(), self.comp.total_out());
        loop {
            let before_out = self.comp.total_out();

            let instant = Instant::now();
            let status = self.comp.compress(&[], &mut self.buffer, flate2::FlushCompress::Finish).unwrap();
            self.duration = self.duration.saturating_add(instant.elapsed());

            match status {
                flate2::Status::BufError => panic!("Failed to flush compress due to BufError"),
                status => {
                    let compressed: usize = (self.comp.total_out() - before_out).try_into().unwrap();

                    if compressed == 0 {
                        break;
                    }

                    let finished = match status {
                        flate2::Status::StreamEnd => true,
                        _ => false,
                    };

                    let header = Header { finished, length: compressed };
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
