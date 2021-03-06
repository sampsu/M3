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

use core::mem;

use m3::boxed::Box;
use m3::col::{BoxRef, BoxList};
use m3::profile;
use m3::test;

#[derive(Default, Clone)]
struct TestItem {
    data: u32,
    prev: Option<BoxRef<TestItem>>,
    next: Option<BoxRef<TestItem>>,
}

impl_boxitem!(TestItem);

impl TestItem {
    pub fn new(data: u32) -> Self {
        TestItem {
            data: data,
            prev: None,
            next: None,
        }
    }
}

pub fn run(t: &mut test::Tester) {
    run_test!(t, push_back);
    run_test!(t, push_front);
    run_test!(t, push_pop);
    run_test!(t, clear);
}

fn push_back() {
    let mut prof = profile::Profiler::new().repeats(30);

    #[derive(Default)]
    struct ListTester(BoxList<TestItem>);

    impl profile::Runner for ListTester {
        fn pre(&mut self) {
            self.0.clear();
        }
        fn run(&mut self) {
            for i in 0..100 {
                self.0.push_back(Box::new(TestItem::new(i)));
            }
        }
    }

    println!("Appending 100 elements: {}", prof.runner_with_id(&mut ListTester::default(), 0x60));
}

fn push_front() {
    let mut prof = profile::Profiler::new().repeats(30);

    #[derive(Default)]
    struct ListTester(BoxList<TestItem>);

    impl profile::Runner for ListTester {
        fn pre(&mut self) {
            self.0.clear();
        }
        fn run(&mut self) {
            for i in 0..100 {
                self.0.push_front(Box::new(TestItem::new(i)));
            }
        }
    }

    println!("Prepending 100 elements: {}", prof.runner_with_id(&mut ListTester::default(), 0x61));
}

fn push_pop() {
    let mut prof = profile::Profiler::new().repeats(30);

    #[derive(Default)]
    struct ListTester(BoxList<TestItem>, Option<Box<TestItem>>, usize);

    impl profile::Runner for ListTester {
        fn pre(&mut self) {
            self.1 = Some(Box::new(TestItem::new(213)));
        }
        fn run(&mut self) {
            let item = mem::replace(&mut self.1, None);
            self.0.push_front(item.unwrap());
        }
        fn post(&mut self) {
            self.2 += 1;
            assert_eq!(self.0.len(), self.2);
        }
    }

    println!("Prepending 1 element: {}", prof.runner_with_id(&mut ListTester::default(), 0x62));
}

fn clear() {
    let mut prof = profile::Profiler::new().repeats(30);

    #[derive(Default)]
    struct ListTester(BoxList<TestItem>);

    impl profile::Runner for ListTester {
        fn pre(&mut self) {
            for i in 0..100 {
                self.0.push_back(Box::new(TestItem::new(i)));
            }
        }
        fn run(&mut self) {
            self.0.clear();
        }
    }

    println!("Clearing 100-element list: {}", prof.runner_with_id(&mut ListTester::default(), 0x63));
}
