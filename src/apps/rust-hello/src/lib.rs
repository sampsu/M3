#![no_std]

#[macro_use]
extern crate m3;

use m3::syscalls;
use m3::time;
use m3::env;
use m3::collections::*;

#[no_mangle]
pub fn main() -> i32 {
    let vec = vec![1, 42, 23];
    println!("my vec:");
    for v in vec {
        println!("  {}", v);
    }

    let mut s: String = format!("my float is {:.3} and my args are", 12.5);
    for a in env::args() {
        s += " ";
        s += a;
    }
    println!("here: {}", s);

    for (i, a) in env::args().enumerate() {
        println!("arg {}: {}", i, a);
    }

    {
        let res = syscalls::create_mgate(5, 0, 0x1000, 0x3);
        println!("res: {:?}", res);
    }

    {
        let res = syscalls::create_mgate(5, 0, 0x1000, 0x3);
        println!("res: {:?}", res);
    }

    let mut total = 0;
    for _ in 0..10 {
        let start = time::start(0);
        syscalls::noop().unwrap();
        let end = time::stop(0);
        total += end - start;
    }
    assert!(total < 10);

    println!("per call: {}", total / 10);

    0
}