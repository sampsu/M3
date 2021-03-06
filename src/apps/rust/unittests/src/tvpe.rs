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

use m3::com::{SendGate, SGateArgs, RecvGate};
use m3::boxed::Box;
use m3::env;
use m3::test;
use m3::util;
use m3::vpe::{Activity, VPE, VPEArgs};

pub fn run(t: &mut test::Tester) {
    run_test!(t, run_arguments);
    run_test!(t, run_send_receive);
    #[cfg(target_os = "none")]
    run_test!(t, exec_fail);
    run_test!(t, exec_hello);
    run_test!(t, exec_rust_hello);
}

fn run_arguments() {
    let mut vpe = assert_ok!(VPE::new_with(VPEArgs::new("test")));

    let act = assert_ok!(vpe.run(Box::new(|| {
        assert_eq!(env::args().count(), 1);
        assert!(env::args().nth(0).is_some());
        assert!(env::args().nth(0).unwrap().ends_with("rustunittests"));
        0
    })));

    assert_eq!(act.wait(), Ok(0));
}

fn run_send_receive() {
    let mut vpe = assert_ok!(VPE::new_with(VPEArgs::new("test")));

    let mut rgate = assert_ok!(RecvGate::new(util::next_log2(256), util::next_log2(256)));
    let sgate = assert_ok!(SendGate::new_with(SGateArgs::new(&rgate).credits(256)));

    assert_ok!(vpe.delegate_obj(rgate.sel()));

    let act = assert_ok!(vpe.run(Box::new(move || {
        assert_ok!(rgate.activate());
        let (i1, i2) = assert_ok!(recv_vmsg!(&rgate, u32, u32));
        assert_eq!((i1, i2), (42, 23));
        (i1 + i2) as i32
    })));

    assert_ok!(send_vmsg!(&sgate, RecvGate::def(), 42, 23));

    assert_eq!(act.wait(), Ok(42 + 23));
}

#[cfg(target_os = "none")]
fn exec_fail() {
    use m3::errors::Code;

    let mut vpe = assert_ok!(VPE::new_with(VPEArgs::new("test")));

    // file too small
    {
        let act = vpe.exec(&["/testfile.txt"]);
        assert!(act.is_err() && act.err().unwrap().code() == Code::EndOfFile);
    }

    // not an ELF file
    {
        let act = vpe.exec(&["/pat.bin"]);
        assert!(act.is_err() && act.err().unwrap().code() == Code::InvalidElf);
    }
}

fn exec_hello() {
    let mut vpe = assert_ok!(VPE::new_with(VPEArgs::new("test")));

    let act = assert_ok!(vpe.exec(&["/bin/hello"]));
    assert_eq!(act.wait(), Ok(0));
}

fn exec_rust_hello() {
    let mut vpe = assert_ok!(VPE::new_with(VPEArgs::new("test")));

    let act = assert_ok!(vpe.exec(&["/bin/rusthello"]));
    assert_eq!(act.wait(), Ok(0));
}
