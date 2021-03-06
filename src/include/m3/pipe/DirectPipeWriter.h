/*
 * Copyright (C) 2016-2018, Nils Asmussen <nils@os.inf.tu-dresden.de>
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

#pragma once

#include <base/Common.h>

#include <m3/vfs/File.h>

namespace m3 {

class DirectPipe;

/**
 * Writes into a previously constructed pipe.
 */
class DirectPipeWriter : public File {
    friend class DirectPipe;

public:
    struct State {
        explicit State(capsel_t caps, size_t size);

        ssize_t find_spot(size_t *len);
        void read_replies();

        MemGate _mgate;
        RecvGate _rgate;
        SendGate _sgate;
        size_t _size;
        size_t _free;
        size_t _rdpos;
        size_t _wrpos;
        int _capacity;
        int _eof;
    };

    explicit DirectPipeWriter(capsel_t caps, size_t size, State *state);

public:
    /**
     * Sends EOF and waits for all outstanding replies
     */
    ~DirectPipeWriter();

    virtual Errors::Code stat(FileInfo &) const override {
        // not supported
        return Errors::NOT_SUP;
    }
    virtual ssize_t seek(size_t, int) override {
        // not supported
        return Errors::NOT_SUP;
    }

    virtual ssize_t read(void *, size_t) override {
        // not supported
        return 0;
    }
    virtual ssize_t write(const void *buffer, size_t count) override {
        return write(buffer, count, true);
    }
    // returns -1 when in non blocking mode and there is not enough space left in buffer
    ssize_t write(const void *buffer, size_t count, bool blocking);

    virtual File *clone() const override {
        return nullptr;
    }

    virtual char type() const override {
        return 'P';
    }
    virtual Errors::Code delegate(VPE &vpe) override;
    virtual void serialize(Marshaller &m) override;
    static File *unserialize(Unmarshaller &um);

private:
    void send_eof();

    capsel_t _caps;
    size_t _size;
    State *_state;
    bool _noeof;
};

}
