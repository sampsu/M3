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

use base::cfg;
use base::dtu::PEId;
use base::goff;
use base::kif::{PEDesc, PEISA, PEType};
use base::libc;
use core::intrinsics;
use core::ptr;

use mem;
use platform;

pub fn init() -> platform::KEnv {
    let mut kenv: platform::KEnv = unsafe { intrinsics::uninit() };

    // no modules
    kenv.mods[0] = 0;

    // init PEs
    kenv.pe_count = cfg::PE_COUNT as u64;
    for i in 0..cfg::PE_COUNT {
        kenv.pes[i] = PEDesc::new(PEType::COMP_IMEM, PEISA::X86, 1024 * 1024).value();
    }

    // create memory
    let base = unsafe {
        libc::mmap(
            ptr::null_mut(),
            cfg::MEM_SIZE,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_ANON | libc::MAP_PRIVATE,
            -1,
            0
        )
    };
    assert!(base != libc::MAP_FAILED);

    let mut m = mem::MemMod::new(kernel_pe(), base as goff, cfg::MEM_SIZE);
    // allocate FS image
    m.allocate(cfg::FS_MAX_SIZE, 1).expect("Not enough space for FS image");
    mem::get().add(m);

    kenv
}

pub fn kernel_pe() -> PEId {
    0
}
pub fn first_user_pe() -> PEId {
    kernel_pe() + 1
}
pub fn last_user_pe() -> PEId {
    platform::pe_count() - 1
}

pub fn default_rcvbuf(_pe: PEId) -> goff {
    0
}
pub fn rcvbufs_size(_pe: PEId) -> usize {
    usize::max_value()
}
