/*
 * Copyright (C) 2018, Nils Asmussen <nils@os.inf.tu-dresden.de>
 * Economic rights: Technische Universitaet Dresden (Germany)
 *
 * This file is part of M3 (Microkernel-based SysteM for Heterogeneous Manycores).
 *
 * M3 is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License version 2 as
 * published by the Free Software Foundation.
 *
 * M3 is distributed in the hope that it will be useful, but
 * WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
 * General Public License version 2 for more details.
 */

use col::{Vec, String};
use core::fmt;
use errors::Error;
use io::{Read, Write};
use util;
use vfs::{SeekMode, Seek};

pub struct BufReader<R : Read> {
    reader: R,
    buf: Vec<u8>,
    pos: usize,
    cap: usize,
}

impl<R : Read> BufReader<R> {
    pub fn new(reader: R) -> Self {
        Self::with_capacity(reader, 512)
    }

    pub fn with_capacity(reader: R, cap: usize) -> Self {
        let mut br = BufReader {
            reader: reader,
            buf: Vec::with_capacity(cap),
            pos: 0,
            cap: 0,
        };
        unsafe { br.buf.set_len(cap) };
        br
    }

    pub fn get_ref(&self) -> &R {
        &self.reader
    }

    pub fn get_mut(&mut self) -> &mut R {
        &mut self.reader
    }

    pub fn read_line(&mut self, s: &mut String) -> Result<usize, Error> {
        let mut total = 0;
        loop {
            let mut buf = [0u8; 1];
            let len = self.read(&mut buf)?;
            if len == 0 || buf[0] == b'\n' {
                break
            }

            s.push(buf[0] as char);
            total += 1;
        }
        Ok(total)
    }
}

impl<R : Read> Read for BufReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        // read directly into user buffer, if the buffer is larger
        if buf.len() > self.buf.capacity() && self.pos == self.cap {
            return self.reader.read(buf)
        }

        if self.pos >= self.cap {
            let end = self.buf.len();
            self.cap = self.reader.read(&mut self.buf.as_mut_slice()[0..end])?;
            self.pos = 0;
        }

        let end = util::min(self.cap, self.pos + buf.len());
        let res = end - self.pos;
        if end > self.pos {
            &buf[0..res].copy_from_slice(&self.buf[self.pos..end]);
        }
        self.pos += res;
        Ok(res)
    }
}

impl<R: Read + Seek> Seek for BufReader<R> {
    fn seek(&mut self, off: usize, whence: SeekMode) -> Result<usize, Error> {
        if whence != SeekMode::CUR || off != 0 {
            // invalidate buffer
            self.pos = 0;
            self.cap = 0;
        }
        self.reader.seek(off, whence)
    }
}

impl<R: Read + fmt::Debug> fmt::Debug for BufReader<R> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "BufReader[reader={:?}, pos={}, cap={}]", self.reader, self.pos, self.cap)
    }
}

pub struct BufWriter<W : Write> {
    writer: W,
    buf: Vec<u8>,
    pos: usize,
}

impl<W : Write> BufWriter<W> {
    pub fn new(writer: W) -> Self {
        Self::with_capacity(writer, 512)
    }

    pub fn with_capacity(writer: W, cap: usize) -> Self {
        let mut br = BufWriter {
            writer: writer,
            buf: Vec::with_capacity(cap),
            pos: 0,
        };
        unsafe { br.buf.set_len(cap) };
        br
    }

    pub fn get_ref(&self) -> &W {
        &self.writer
    }

    pub fn get_mut(&mut self) -> &mut W {
        &mut self.writer
    }

    fn do_flush(&mut self) -> Result<(), Error> {
        if self.pos > 0 {
            self.writer.write(&self.buf[0..self.pos])?;
            self.pos = 0;
        }
        Ok(())
    }
}

impl<W : Write> Write for BufWriter<W> {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        // write directly from user buffer, if it is larger
        if buf.len() > self.buf.len() {
            self.do_flush()?;

            self.writer.write(buf)
        }
        else {
            let end = util::min(self.buf.len(), self.pos + buf.len());
            let res = end - self.pos;
            if end > self.pos {
                &self.buf[self.pos..end].copy_from_slice(&buf[0..res]);
            }

            self.pos += res;

            // use line buffering
            if self.buf.iter().find(|b| **b == b'\n').is_some() {
                self.flush()?;
            }
            else if self.pos == self.buf.len() {
                self.do_flush()?;
            }

            Ok(res)
        }
    }

    fn flush(&mut self) -> Result<(), Error> {
        self.do_flush()?;
        self.writer.flush()
    }
}

impl<W: Write + Seek> Seek for BufWriter<W> {
    fn seek(&mut self, off: usize, whence: SeekMode) -> Result<usize, Error> {
        if whence != SeekMode::CUR || off != 0 {
            self.flush()?;
        }
        self.writer.seek(off, whence)
    }
}

impl<W: Write> Drop for BufWriter<W> {
    fn drop(&mut self) {
        self.flush().unwrap();
    }
}

impl<W: Write + fmt::Debug> fmt::Debug for BufWriter<W> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "BufWriter[writer={:?}, pos={}]", self.writer, self.pos)
    }
}
