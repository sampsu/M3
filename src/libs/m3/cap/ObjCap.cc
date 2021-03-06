/*
 * Copyright (C) 2016-2017, Nils Asmussen <nils@os.inf.tu-dresden.de>
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

#include <m3/ObjCap.h>
#include <m3/Syscalls.h>
#include <m3/VPE.h>

namespace m3 {

void ObjCap::release() {
    if(_sel != INVALID) {
        if(~_flags & KEEP_CAP)
            VPE::self().revoke(KIF::CapRngDesc(KIF::CapRngDesc::OBJ, sel()));
    }
}

}
