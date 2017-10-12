/*
 * Copyright (C) 2015, Nils Asmussen <nils@os.inf.tu-dresden.de>
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

#include <m3/session/M3FS.h>
#include <m3/stream/FStream.h>
#include <m3/vfs/VFS.h>

namespace m3 {

FStream::FStream(int fd, int perms, size_t bufsize, uint flags)
    : IStream(), OStream(), _fd(fd), _fpos(),
      _rbuf(file()->create_buf((perms & FILE_R) ? bufsize : 0)),
      _wbuf(file()->create_buf((perms & FILE_W) ? bufsize : 0)),
      _flags(FL_DEL_BUF | flags) {
}

FStream::FStream(const char *filename, int perms, size_t bufsize)
    : FStream(filename, bufsize, bufsize, perms) {
    if(_fd == FileTable::INVALID)
        _state |= FL_ERROR;
}

FStream::FStream(const char *filename, size_t rsize, size_t wsize, int perms)
    : IStream(), OStream(), _fd(VFS::open(filename, get_perms(perms))), _fpos(),
      _rbuf(_fd != FileTable::INVALID ? file()->create_buf((perms & FILE_R) ? rsize : 0) : nullptr),
      _wbuf(_fd != FileTable::INVALID ? file()->create_buf((perms & FILE_W) ? wsize : 0) : nullptr),
      _flags(FL_DEL_BUF | FL_DEL_FILE) {
    if(_fd == FileTable::INVALID)
        _state |= FL_ERROR;
}

FStream::~FStream() {
    flush();
    if(!(_flags & FL_DEL_BUF)) {
        if(_rbuf)
            _rbuf->buffer = nullptr;
        if(_wbuf)
            _wbuf->buffer = nullptr;
    }
    if((_flags & FL_DEL_FILE) && _fd != FileTable::INVALID)
        VFS::close(_fd);
    delete _rbuf;
    delete _wbuf;
}

void FStream::set_error(ssize_t res) {
    if(res < 0)
        _state |= FL_ERROR;
    else if(res == 0)
        _state |= FL_EOF;
}

size_t FStream::read(void *dst, size_t count) {
    if(bad())
        return 0;

    // ensure that our write-buffer is empty
    // TODO maybe it's better to have just one buffer for both and track dirty regions?
    flush();

    // simply use the unbuffered read, if the buffer is empty and all is aligned
    if(_rbuf->empty() && Math::is_aligned(_fpos, DTU_PKG_SIZE) &&
                         Math::is_aligned(dst, DTU_PKG_SIZE) &&
                         Math::is_aligned(count, DTU_PKG_SIZE)) {
        ssize_t res = file()->read(dst, count);
        if(res > 0)
            _fpos += static_cast<size_t>(res);
        else
            set_error(res);
        return res < 0 ? 0 : static_cast<size_t>(res);
    }

    if(!_rbuf->buffer) {
        _state |= FL_ERROR;
        return 0;
    }

    size_t total = 0;
    char *buf = reinterpret_cast<char*>(dst);
    while(count > 0) {
        ssize_t res = _rbuf->read(file(), _fpos + total, buf + total, count);
        if(res <= 0) {
            set_error(res);
            return total;
        }
        total += static_cast<size_t>(res);
        count -= static_cast<size_t>(res);
    }

    _fpos += total;
    return total;
}

void FStream::flush() {
    if(_wbuf)
        set_error(_wbuf->flush(file()));
}

size_t FStream::seek(size_t offset, int whence) {
    if(error())
        return 0;

    if(whence != M3FS_SEEK_CUR || offset != 0) {
        // TODO for simplicity, we always flush the write-buffer if we're changing the position
        flush();
    }

    // if we seek within our read-buffer, it's enough to set the position
    size_t newpos = offset;
    int res = _rbuf->seek(_fpos, whence, newpos);
    if(res > 0) {
        _fpos = newpos;
        return _fpos;
    }
    // not supported?
    if(res < 0) {
        _state |= FL_ERROR;
        return 0;
    }

    // File::seek assumes that it is aligned. _fpos needs to reflect the actual position, of course
    size_t posoff = newpos & (DTU_PKG_SIZE - 1);
    _fpos = file()->seek(newpos - posoff, whence) + posoff;
    _rbuf->invalidate();
    return _fpos;
}

size_t FStream::write(const void *src, size_t count) {
    if(bad())
        return 0;

    // simply use the unbuffered write, if the buffer is empty and all is aligned
    if(_wbuf->empty() && Math::is_aligned(_fpos, DTU_PKG_SIZE) &&
                         Math::is_aligned(src, DTU_PKG_SIZE) &&
                         Math::is_aligned(count, DTU_PKG_SIZE)) {
        ssize_t res = file()->write(src, count);
        set_error(res);
        return res < 0 ? 0 : static_cast<size_t>(res);
    }

    if(!_wbuf->buffer) {
        _state |= FL_ERROR;
        return 0;
    }

    const char *buf = reinterpret_cast<const char*>(src);
    size_t total = 0;
    while(count > 0) {
        ssize_t res = _wbuf->write(file(), _fpos + total, buf + total, count);
        if(res <= 0) {
            set_error(res);
            return 0;
        }

        total += static_cast<size_t>(res);
        count -= static_cast<size_t>(res);

        if(count || ((_flags & FL_LINE_BUF) && buf[total - 1] == '\n'))
            flush();
    }

    _fpos += total;
    return total;
}

}
