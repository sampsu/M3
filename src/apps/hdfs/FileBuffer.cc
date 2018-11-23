#include "FileBuffer.h"

#include <fs/internal.h>

using namespace m3;

FileBufferHead::FileBufferHead(blockno_t bno, size_t size, size_t blocksize)
    : BufferHead(bno, size),
      _data(MemGate::create_global(size * blocksize, MemGate::RWX)) {
    _extents.append(new InodeExt(bno, size));
}

FileBuffer::FileBuffer(size_t blocksize, DiskSession *disk, size_t max_load)
    : Buffer(blocksize, disk),
      _max_load(max_load) {
}

size_t FileBuffer::get_extent(blockno_t bno, size_t size, capsel_t sel, int perms, size_t accessed,
                              bool load, bool check) {
    while(true) {
        FileBufferHead *b = FileBuffer::get(bno);
        if(b) {
            if(b->locked) {
                // wait
                SLOG(FS, "FileFuffer: Waiting for cached blocks <"
                    << b->key() << "," << b->_size << ">, for block " << bno);
                ThreadManager::get().wait_for(b->unlock);
            }
            else {
                // lock?
                lru.remove(b);
                lru.append(b);
                SLOG(FS, "FileFuffer: Found cached blocks <"
                    << b->key() << "," << b->_size << ">, for block " << bno);
                size_t len       = Math::min(size, (b->_size - (bno - b->key())));
                Errors::Code res = m3::Syscalls::get().derivemem(
                    sel, b->_data.sel(), (bno - b->key()) * _blocksize, len * _blocksize, perms);

                if(res != Errors::NONE)
                    return 0;
                return len * _blocksize;
            }
        }
        else
            break;
    }

    if(check)
        return 0;

    // load chunk into memory
    // size_t max_size = Math::min((size_t)FILE_BUFFER_SIZE, _max_load);
    size_t max_size = (size_t)Math::min(FILE_BUFFER_SIZE, (1 << (accessed)));
    // size_t max_size = Math::min((size_t)FILE_BUFFER_SIZE, _max_load * accessed);
    size_t load_size = 0;
    load ? load_size = Math::min(size, max_size) : load_size = Math::min((size_t)FILE_BUFFER_SIZE, size);
    FileBufferHead *b;
    if((_size + load_size) > FILE_BUFFER_SIZE) {
        do {
            b = FileBuffer::get(lru.begin()->key());
            if(b->locked) {
                // wait
                SLOG(FS, "FileBuffer: Waiting for eviction of block <" << b->key() << ">");
                ThreadManager::get().wait_for(b->unlock);
            }
            else {
                SLOG(FS, "FileBuffer: Evicting block <" << b->key() << ">");
                lru.removeFirst();
                ht.remove(b);
                if(b->dirty)
                    flush_chunk(b);
                // revoke all subsets
                VPE::self().revoke(KIF::CapRngDesc(KIF::CapRngDesc::OBJ, b->_data.sel(), 1));
                _size -= b->_size;
                delete b;
            }
        }
        while((_size + load_size) > FILE_BUFFER_SIZE);
    }

    b = new FileBufferHead(bno, load_size, _blocksize);

    _size += b->_size;
    ht.insert(b);
    lru.append(b);
    // load from disk
    SLOG(FS, "FileBuffer: Allocating block <" << b->key() << ">" << (load ? " : loading" : ""));
    KIF::CapRngDesc crd(KIF::CapRngDesc::OBJ, b->_data.sel(), 1);
    KIF::ExchangeArgs args;
    args.count = 2;
    args.vals[0] = static_cast<xfer_t>(b->key());
    args.vals[1] = static_cast<xfer_t>(b->_size);
    _disk->delegate(crd, &args);

    if(load)
        _disk->read(b->key(), b->key(), b->_size, _blocksize);

    b->locked = false;
    ThreadManager::get().notify(b->unlock);

    Errors::Code res = Syscalls::get().derivemem(sel, b->_data.sel(), 0, load_size * _blocksize, perms);
    if(res != Errors::NONE)
        return 0;
    return load_size * _blocksize;
}

FileBufferHead *FileBuffer::get(blockno_t bno) {
    FileBufferHead *b = reinterpret_cast<FileBufferHead*>(ht.find(bno));
    if(b)
        return b;
    return nullptr;
}

void FileBuffer::flush_chunk(BufferHead *b) {
    b->locked = true;

    SLOG(FS, "FileBuffer: Write back block <" << b->key() << ">");
    _disk->write(b->key(), b->key(), b->_size, _blocksize);

    b->dirty  = false;
    b->locked = false;
    ThreadManager::get().notify(b->unlock);
}

void FileBuffer::flush() {
    while(!ht.empty()) {
        FileBufferHead *b = reinterpret_cast<FileBufferHead *>(ht.remove_root());
        if(b->dirty)
            flush_chunk(b);
    }
}