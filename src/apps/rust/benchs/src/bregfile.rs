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

use m3::io::Read;
use m3::profile;
use m3::test;
use m3::vfs::{OpenFlags, VFS};

pub fn run(t: &mut test::Tester) {
    run_test!(t, read);
    run_test!(t, write);
}

fn read() {
    let mut buf = vec![0u8; 8192];

    let mut prof = profile::Profiler::new().repeats(2).warmup(1);

    println!("2 MiB file with 8K buf: {}", prof.run_with_id(|| {
        let mut file = assert_ok!(VFS::open("/data/2048k.txt", OpenFlags::R));
        loop {
            let amount = assert_ok!(file.read(&mut buf));
            if amount == 0 {
                break;
            }
        }
    }, 0x20));
}

fn write() {
    const SIZE: usize = 2 * 1024 * 1024;
    let buf = vec![0u8; 8192];

    let mut prof = profile::Profiler::new().repeats(2).warmup(1);

    println!("2 MiB file with 8K buf: {}", prof.run_with_id(|| {
        let mut file = assert_ok!(VFS::open("/newfile",
            OpenFlags::W | OpenFlags::CREATE | OpenFlags::TRUNC));

        let mut total = 0;
        while total < SIZE {
            let amount = assert_ok!(file.write(&buf));
            if amount == 0 {
                break;
            }
            total += amount;
        }
    }, 0x21));
}
