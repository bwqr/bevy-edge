use std::io::{Write, Read};
use log::debug;

pub struct Compressor<W> {
    dest: W,
    comp: flate2::Compress,
}

impl<W> Compressor<W> {
    pub fn new(dest: W, level: u32) -> Self {
        Compressor {
            dest,
            comp: flate2::Compress::new(flate2::Compression::new(level), false),
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
}

impl<R> Decompressor<R> {
    pub fn new(source: R) -> Self {
        Decompressor {
            source,
            decomp: flate2::Decompress::new(false),
            buffer: vec![0u8; 1024 * 32],
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
}

impl<R> Decompressor<R> {
    fn remaining(&self) -> usize {
        self.end - self.start
    }
}

impl<R> Read for Decompressor<R> where R: Read {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.remaining() == 0 {
            self.start = 0;
            self.end = 0;

            self.end = self.source.read(&mut self.buffer).unwrap();
        }

        // If source did not read anything, directly return 0
        if self.remaining() == 0 {
            return Ok(0);
        }

        debug!("reading buf {}", buf.len());
        let (before_in, before_out) = (self.decomp.total_in(), self.decomp.total_out());
        debug!("reading before {before_in}, {before_out}");

        match self.decomp.decompress(&self.buffer[self.start..self.end], buf, flate2::FlushDecompress::None) {
            Ok(flate2::Status::Ok) | Ok(flate2::Status::StreamEnd) => { },
            Ok(status) => panic!("StatusError {status:?}"),
            Err(e) => panic!("DecompressError {e:?}"),
        }

        let (after_in, after_out) = (self.decomp.total_in(), self.decomp.total_out());
        debug!("reading after {after_in}, {after_out}");

        self.start += <u64 as TryInto<usize>>::try_into(after_in - before_in).unwrap();

        Ok((after_out - before_out).try_into().unwrap())
    }
}

impl<W> Write for Compressor<W> where W: Write {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut comp_buf = [0u8; 1024 * 32];

        debug!("writing buf {}", buf.len());
        let (before_in, before_out) = (self.comp.total_in(), self.comp.total_out());
        debug!("writing before {before_in}, {before_out}");

        match self.comp.compress(buf, &mut comp_buf, flate2::FlushCompress::None).unwrap() {
            flate2::Status::Ok => { },
            status => panic!("Failed to compress {status:?}"),
        }

        let (after_in, after_out) = (self.comp.total_in(), self.comp.total_out());
        debug!("writing after {after_in}, {after_out}");

        let should_written: usize = (after_out - before_out).try_into().unwrap();
        self.dest.write_all(&comp_buf[..should_written]).unwrap();

        debug!("return written {}", after_in - before_in);
        Ok((after_in - before_in).try_into().unwrap())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        debug!("flushing, {}, {}", self.comp.total_in(), self.comp.total_out());
        let mut vec = vec![0u8; 1024 * 32];
        loop {
            let before_out = self.comp.total_out();
            self.comp.compress(&[], &mut vec, flate2::FlushCompress::Finish).unwrap();
            let compressed: usize = (self.comp.total_out() - before_out).try_into().unwrap();

            if compressed == 0 {
                break
            } else if compressed >= vec.len() {
                self.dest.write_all(&vec).unwrap();
            } else {
                self.dest.write_all(&vec[..compressed + 1]).unwrap();
            }

        }

        self.dest.flush().unwrap();
        debug!("flushed, {}, {}", self.comp.total_in(), self.comp.total_out());
        Ok(())
    }
}
