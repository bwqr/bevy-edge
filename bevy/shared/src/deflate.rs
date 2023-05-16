use std::io::{Write, Read};
use log::trace;

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
}

impl<R> Decompressor<R> {
    pub fn new(source: R) -> Self {
        Decompressor {
            source,
            decomp: flate2::Decompress::new(true),
            buffer: vec![0u8; 1024 * 8],
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

impl<R: Read> Decompressor<R> {
    pub fn finish(&mut self) -> std::io::Result<()> {
        let mut buf = [0u8; 128];
        match self.decomp.decompress(&[], &mut buf, flate2::FlushDecompress::Finish) {
            Ok(flate2::Status::BufError) => panic!("Buf Error"),
            Ok(flate2::Status::StreamEnd) => {
                trace!("Stream end in finish");
                Ok(())
            }
            Ok(flate2::Status::Ok) => {
                trace!("It is Ok in finish");
                Ok(())
            },
            Err(_) => {
                self.start = 0;
                self.end = self.source.read(&mut self.buffer[self.end..])
                    .map_err(|e| std::io::Error::new(e.kind(), "failed to read from source".to_string()))?;
                trace!("read from source in finish {}", self.end);
                Ok(())
            }
        }
    }
}

impl<R> Read for Decompressor<R> where R: Read {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        trace!("reading buf {}", buf.len());
        let (before_in, before_out) = (self.decomp.total_in(), self.decomp.total_out());
        trace!("reading before {before_in}, {before_out}");

        match self.decomp.decompress(&self.buffer[self.start..self.end], buf, flate2::FlushDecompress::None) {
            Ok(flate2::Status::Ok) => { },
            Ok(flate2::Status::StreamEnd) => trace!("Stream is ended"),
            Ok(flate2::Status::BufError) => {
                self.buffer.copy_within(self.start..self.end, 0);
                self.end = self.end - self.start;
                self.start = 0;
                self.end += self.source.read(&mut self.buffer[self.end..])
                    .map_err(|e| std::io::Error::new(e.kind(), "failed to read from source".to_string()))?;
                trace!("read from source {}", self.end);

                if self.end == 0 {
                    return Ok(0);
                }

                return self.read(buf);
            }
            Err(e) => {
                trace!("content of buffer {:?}", &self.buffer[self.start..self.end]);
                return Err(std::io::Error::new(std::io::ErrorKind::Other, format!("DecompressError {e:?}, start {}, end {}", self.start, self.end)));
            }
        }

        let (after_in, after_out) = (self.decomp.total_in(), self.decomp.total_out());
        trace!("reading after {after_in}, {after_out}");

        self.start += <u64 as TryInto<usize>>::try_into(after_in - before_in).unwrap();

        Ok((after_out - before_out).try_into().unwrap())
    }
}

impl<W> Write for Compressor<W> where W: Write {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        trace!("writing buf {}", buf.len());
        let (before_in, before_out) = (self.comp.total_in(), self.comp.total_out());
        trace!("writing before {before_in}, {before_out}");

        match self.comp.compress(buf, &mut self.buffer, flate2::FlushCompress::None).unwrap() {
            flate2::Status::Ok => { },
            status => panic!("Failed to compress {status:?}"),
        }

        let (after_in, after_out) = (self.comp.total_in(), self.comp.total_out());
        trace!("writing after {after_in}, {after_out}");

        let should_written: usize = (after_out - before_out).try_into().unwrap();
        self.dest.write_all(&self.buffer[..should_written]).unwrap();

        trace!("return written {}", after_in - before_in);

        if after_in - before_in == 0 {
            return self.write(buf);
        }

        Ok((after_in - before_in).try_into().unwrap())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        trace!("flushing, {}, {}", self.comp.total_in(), self.comp.total_out());
        loop {
            trace!("looping in flush");
            let before_out = self.comp.total_out();
            match self.comp.compress(&[], &mut self.buffer, flate2::FlushCompress::Finish) {
                Ok(flate2::Status::BufError) => panic!("Failed to flush compress due to BufError"),
                Ok(_) => {
                    let compressed: usize = (self.comp.total_out() - before_out).try_into().unwrap();

                    if compressed == 0 {
                        break;
                    }

                    self.dest.write_all(&self.buffer[..compressed]).unwrap();
                },
                Err(e) => return Err(std::io::Error::new(std::io::ErrorKind::Other, format!("CompressError {e:?}"))),
            }
        }

        self.dest.flush().unwrap();
        trace!("flushed, {}, {}", self.comp.total_in(), self.comp.total_out());
        Ok(())
    }
}
