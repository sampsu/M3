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

#pragma once

#include <base/Common.h>
#include <base/col/SList.h>
#include <base/col/Treap.h>
#include <base/stream/OStream.h>
#include <base/Errors.h>

#include <m3/session/Session.h>

#include "RegionList.h"

class AddrSpace;

class DataSpace : public m3::TreapNode<DataSpace, uintptr_t>, public m3::SListItem {
public:
    explicit DataSpace(AddrSpace *as, uintptr_t addr, size_t size, uint flags)
        : TreapNode(addr), SListItem(), _as(as), _id(_next_id++), _flags(flags),
          _regs(this), _size(size) {
    }
    virtual ~DataSpace() {
    }

    bool matches(uintptr_t k) {
        return k >= addr() && k < addr() + _size;
    }

    ulong id() const {
        return _id;
    }
    AddrSpace *addrspace() {
        return _as;
    }
    uint flags() const {
        return _flags;
    }
    uintptr_t addr() const {
        return key();
    }
    size_t size() const {
        return _size;
    }

    virtual const char *type() const = 0;
    virtual m3::Errors::Code handle_pf(uintptr_t virt) = 0;
    virtual DataSpace *clone(AddrSpace *as) = 0;

    void inherit(DataSpace *ds);

    void print(m3::OStream &os) const;

protected:
    AddrSpace *_as;
    ulong _id;
    uint _flags;
    RegionList _regs;
    size_t _size;
    static ulong _next_id;
};

class AnonDataSpace : public DataSpace {
public:
    static constexpr size_t MAX_PAGES = 4;

    explicit AnonDataSpace(AddrSpace *as, uintptr_t addr, size_t size, uint flags)
        : DataSpace(as, addr, size, flags) {
    }

    const char *type() const override {
        return "Anon";
    }
    DataSpace *clone(AddrSpace *as) override {
        return new AnonDataSpace(as, addr(), size(), _flags);
    }

    m3::Errors::Code handle_pf(uintptr_t vaddr) override;
};

class ExternalDataSpace : public DataSpace {
public:
    static constexpr size_t MAX_PAGES = 8;

    explicit ExternalDataSpace(AddrSpace *as, uintptr_t addr, size_t size, uint flags, int _id,
            size_t _fileoff, capsel_t sess)
        : DataSpace(as, addr, size, flags), sess(sess), id(_id), fileoff(_fileoff) {
    }
    explicit ExternalDataSpace(AddrSpace *as, uintptr_t addr, size_t size, uint flags, int _id,
            size_t _fileoff)
        : DataSpace(as, addr, size, flags), sess(m3::VPE::self().alloc_cap()),
          id(_id), fileoff(_fileoff) {
    }

    const char *type() const override {
        return "External";
    }
    DataSpace *clone(AddrSpace *as) override {
        return new ExternalDataSpace(as, addr(), size(), _flags, id, fileoff, sess.sel());
    }

    m3::Errors::Code handle_pf(uintptr_t vaddr) override;

    m3::Session sess;
    int id;
    size_t fileoff;
};
