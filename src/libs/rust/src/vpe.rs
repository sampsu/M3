use boxed::Box;
use alloc::boxed::FnBox;
use cap::{Capability, Flags, Selector};
use cell::RefCell;
use com::{MemGate, RBufSpace};
use collections::Vec;
use core::iter;
use dtu::{EP_COUNT, FIRST_FREE_EP, EpId};
use env;
use elf;
use errors::Error;
use heap;
use kif;
use kif::{cap, CapRngDesc, INVALID_SEL, PEDesc};
use rc::Rc;
use session::M3FS;
use vfs::{BufReader, FileSystem, OpenFlags, Read, RegularFile, Seek, SeekMode};
use libc;
use syscalls;
use util;

#[derive(Debug)]
pub struct VPE {
    cap: Capability,
    pe: PEDesc,
    mem: MemGate,
    next_sel: Selector,
    eps: u64,
    rbufs: RBufSpace,
}

pub struct VPEArgs<'n> {
    name: &'n str,
    pe: PEDesc,
    muxable: bool,
}

pub trait Activity {
    fn vpe(&self) -> &VPE;

    fn start(&self) -> Result<(), Error> {
        syscalls::vpe_ctrl(self.vpe().sel(), kif::syscalls::VPEOp::Start, 0).map(|_| ())
    }

    fn stop(&self) -> Result<(), Error> {
        syscalls::vpe_ctrl(self.vpe().sel(), kif::syscalls::VPEOp::Stop, 0).map(|_| ())
    }

    fn wait(&self) -> Result<i32, Error> {
        syscalls::vpe_ctrl(self.vpe().sel(), kif::syscalls::VPEOp::Wait, 0)
    }
}

pub struct ClosureActivity<'v> {
    vpe: &'v VPE,
    _closure: env::Closure,
}

impl<'v> ClosureActivity<'v> {
    pub fn new<'vo : 'v>(vpe: &'vo VPE, closure: env::Closure) -> ClosureActivity<'v> {
        ClosureActivity {
            vpe: vpe,
            _closure: closure,
        }
    }
}

impl<'v> Activity for ClosureActivity<'v> {
    fn vpe(&self) -> &VPE {
        self.vpe
    }
}

impl<'v> Drop for ClosureActivity<'v> {
    fn drop(&mut self) {
        self.stop().ok();
    }
}

pub struct ExecActivity<'v> {
    vpe: &'v VPE,
    _file: BufReader<RegularFile>,
}

impl<'v> ExecActivity<'v> {
    pub fn new<'vo : 'v>(vpe: &'vo VPE, file: BufReader<RegularFile>) -> ExecActivity<'v> {
        ExecActivity {
            vpe: vpe,
            _file: file,
        }
    }
}

impl<'v> Activity for ExecActivity<'v> {
    fn vpe(&self) -> &VPE {
        self.vpe
    }
}

impl<'v> Drop for ExecActivity<'v> {
    fn drop(&mut self) {
        self.stop().ok();
    }
}

impl<'n> VPEArgs<'n> {
    pub fn new(name: &'n str) -> VPEArgs<'n> {
        VPEArgs {
            name: name,
            pe: VPE::cur().pe(),
            muxable: false,
        }
    }

    pub fn pe(mut self, pe: PEDesc) -> Self {
        self.pe = pe;
        self
    }

    pub fn muxable(mut self, muxable: bool) -> Self {
        self.muxable = muxable;
        self
    }
}

// TODO move
const RT_START: usize   = 0x6000;
const STACK_TOP: usize  = 0xC000;

static mut CUR: Option<VPE> = None;

impl VPE {
    fn new_cur() -> Self {
        VPE {
            cap: Capability::new(0, Flags::KEEP_CAP),
            pe: env::data().pedesc.clone(),
            mem: MemGate::new_bind(1),
            // 0 and 1 are reserved for VPE cap and mem cap
            next_sel: 2,
            eps: 0,
            rbufs: RBufSpace::new(env::data().rbuf_cur as usize, env::data().rbuf_end as usize),
        }
    }

    pub fn cur() -> &'static mut VPE {
        unsafe {
            CUR.as_mut().unwrap()
        }
    }

    pub fn new(name: &str) -> Result<Self, Error> {
        Self::new_with(VPEArgs::new(name))
    }

    pub fn new_with(builder: VPEArgs) -> Result<Self, Error> {
        let sels = VPE::cur().alloc_caps(2);
        let pe = try!(syscalls::create_vpe(
            sels + 0, sels + 1, INVALID_SEL, INVALID_SEL, builder.name,
            builder.pe, 0, 0, builder.muxable
        ));

        Ok(VPE {
            cap: Capability::new(sels + 0, Flags::empty()),
            pe: pe,
            mem: MemGate::new_bind(sels + 1),
            next_sel: 2,
            eps: 0,
            rbufs: RBufSpace::new(0, 0),
        })
    }

    pub fn sel(&self) -> Selector {
        self.cap.sel()
    }
    pub fn pe(&self) -> PEDesc {
        self.pe
    }
    pub fn mem(&self) -> &MemGate {
        &self.mem
    }

    pub fn alloc_cap(&mut self) -> Selector {
        self.alloc_caps(1)
    }
    pub fn alloc_caps(&mut self, count: u32) -> Selector {
        self.next_sel += count;
        self.next_sel - count
    }

    pub fn alloc_ep(&mut self) -> Result<EpId, Error> {
        for ep in 0..EP_COUNT {
            if self.is_ep_free(ep) {
                self.eps |= 1 << ep;
                return Ok(ep)
            }
        }
        Err(Error::NoSpace)
    }

    pub fn is_ep_free(&self, ep: EpId) -> bool {
        (self.eps & (1 << ep)) == 0
    }

    pub fn free_ep(&mut self, ep: EpId) {
        self.eps &= !(1 << ep);
    }

    pub fn rbufs(&mut self) -> &mut RBufSpace {
        &mut self.rbufs
    }

    pub fn delegate_obj(&mut self, sel: Selector) -> Result<(), Error> {
        self.delegate(CapRngDesc::new_from(cap::Type::OBJECT, sel, 1))
    }
    pub fn delegate(&mut self, crd: CapRngDesc) -> Result<(), Error> {
        let start = crd.start();
        self.delegate_to(crd, start)
    }
    pub fn delegate_to(&mut self, crd: CapRngDesc, dst: Selector) -> Result<(), Error> {
        try!(syscalls::exchange(self.sel(), crd, dst, false));
        self.next_sel = util::max(self.next_sel, dst + crd.count());
        Ok(())
    }

    pub fn obtain(&mut self, crd: CapRngDesc) -> Result<(), Error> {
        let count = crd.count();
        let start = self.alloc_caps(count);
        self.obtain_to(crd, start)
    }

    pub fn obtain_to(&mut self, crd: CapRngDesc, dst: Selector) -> Result<(), Error> {
        let own = CapRngDesc::new_from(crd.cap_type(), dst, crd.count());
        syscalls::exchange(self.sel(), own, crd.start(), true)
    }

    pub fn revoke(&mut self, crd: CapRngDesc, del_only: bool) -> Result<(), Error> {
        syscalls::revoke(self.sel(), crd, !del_only)
    }

    pub fn run<F>(&mut self, func: Box<F>) -> Result<ClosureActivity, Error>
                  where F: FnBox() -> i32, F: Send + 'static {
        let get_sp = || {
            let res: usize;
            unsafe {
                asm!("mov %rsp, $0" : "=r"(res));
            }
            res
        };

        let env = env::data();
        let mut senv = env::EnvData::new();

        senv.sp = get_sp() as u64;
        senv.entry = try!(self.copy_regions(senv.sp as usize)) as u64;
        senv.lambda = 1;
        senv.exit_addr = 0;

        senv.pedesc = self.pe();
        senv.heap_size = env.heap_size;

        senv.rbuf_cur = self.rbufs.cur as u64;
        senv.rbuf_end = self.rbufs.end as u64;
        senv.caps = self.next_sel as u64;

        // TODO
        // senv.mounts_len = 0;
        // senv.mounts = reinterpret_cast<uintptr_t>(_ms);
        // senv.fds_len = 0;
        // senv.fds = reinterpret_cast<uintptr_t>(_fds);
        // senv.eps = reinterpret_cast<uintptr_t>(_eps);
        // senv.pager_sgate = 0;
        // senv.pager_rgate = 0;
        // senv.pager_sess = 0;
        // senv._backend = env()->_backend;

        // env goes first
        let mut off = RT_START + util::size_of_val(&senv);

        // create and write closure
        let closure = env::Closure::new(func);
        try!(self.mem.write_obj(&closure, off));
        off += util::size_of_val(&closure);

        // write args
        senv.argc = env.argc;
        senv.argv = try!(self.write_arguments(off, env::args())) as u64;

        // write start env to PE
        try!(self.mem.write_obj(&senv, RT_START));

        // go!
        let act = ClosureActivity::new(self, closure);
        act.start().map(|_| act)
    }

    pub fn exec<S: AsRef<str>>(&mut self, fs: Rc<RefCell<M3FS>>,
                               args: &[S]) -> Result<ExecActivity, Error> {
        let file = try!(fs.borrow_mut().open(args[0].as_ref(), OpenFlags::RX));
        let mut file = BufReader::new(file);

        let mut senv = env::EnvData::new();

        senv.entry = try!(self.load_program(&mut file)) as u64;
        senv.lambda = 0;
        senv.exit_addr = 0;
        senv.sp = STACK_TOP as u64;

        // write args
        let argoff = RT_START + util::size_of_val(&senv);
        senv.argc = args.len() as u32;
        senv.argv = try!(self.write_arguments(argoff, args)) as u64;

        // TODO mounts, fds, and eps

        senv.rbuf_cur = self.rbufs.cur as u64;
        senv.rbuf_end = self.rbufs.end as u64;
        senv.caps = self.next_sel as u64;

        // TODO pager

        senv.pedesc = self.pe();
        senv.heap_size = 0;

        // write start env to PE
        try!(self.mem.write_obj(&senv, RT_START));

        // go!
        let act = ExecActivity::new(self, file);
        act.start().map(|_| act)
    }

    fn copy_regions(&self, sp: usize) -> Result<usize, Error> {
        extern {
            static _text_start: u8;
            static _text_end: u8;
            static _data_start: u8;
            static _bss_end: u8;
            static heap_end: usize;
        }

        let addr = |sym: &u8| {
            (sym as *const u8) as usize
        };

        unsafe {
            // copy text
            let text_start = addr(&_text_start);
            let text_end = addr(&_text_end);
            try!(self.mem.write_bytes(&_text_start, text_end - text_start, text_start));

            // copy data and heap
            let data_start = addr(&_data_start);
            try!(self.mem.write_bytes(&_data_start, libc::heap_used_end() - data_start, data_start));

            // copy end-area of heap
            let heap_area_size = util::size_of::<heap::HeapArea>();
            try!(self.mem.write_bytes(heap_end as *const u8, heap_area_size, heap_end));

            // copy stack
            try!(self.mem.write_bytes(sp as *const u8, STACK_TOP - sp, sp));

            Ok(text_start)
        }
    }

    fn clear_mem(&self, buf: &mut [u8], mut count: usize, mut dst: usize) -> Result<(), Error> {
        if count == 0 {
            return Ok(())
        }

        for i in 0..buf.len() {
            buf[i] = 0;
        }

        while count > 0 {
            let amount = util::min(count, buf.len());
            try!(self.mem.write(&buf[0..amount], dst));
            count -= amount;
            dst += amount;
        }

        Ok(())
    }

    fn load_segment(&self, file: &mut BufReader<RegularFile>,
                    phdr: &elf::Phdr, buf: &mut [u8]) -> Result<(), Error> {
        try!(file.seek(phdr.offset, SeekMode::SET));

        let mut count = phdr.filesz;
        let mut segoff = phdr.vaddr;
        while count > 0 {
            let amount = util::min(count, buf.len());
            let amount = try!(file.read(&mut buf[0..amount]));

            try!(self.mem.write(&buf[0..amount], segoff));

            count -= amount;
            segoff += amount;
        }

        self.clear_mem(buf, phdr.memsz - phdr.filesz, segoff)
    }

    fn load_program(&self, file: &mut BufReader<RegularFile>) -> Result<usize, Error> {
        let mut buf = Box::new([0u8; 4096]);
        let hdr: elf::Ehdr = try!(file.read_object());

        if hdr.ident[0] != '\x7F' as u8 ||
           hdr.ident[1] != 'E' as u8 ||
           hdr.ident[2] != 'L' as u8 ||
           hdr.ident[3] != 'F' as u8 {
            return Err(Error::InvalidElf)
        }

        // copy load segments to destination PE
        let mut off = hdr.phoff;
        for _ in 0..hdr.phnum {
            // load program header
            try!(file.seek(off, SeekMode::SET));
            let phdr: elf::Phdr = try!(file.read_object());

            // we're only interested in non-empty load segments
            if phdr.ty != elf::PT::LOAD.val || phdr.memsz == 0 {
                continue;
            }

            try!(self.load_segment(file, &phdr, &mut *buf));
            off += hdr.phentsize as usize;
        }

        Ok(hdr.entry)
    }

    fn write_arguments<I, S>(&self, off: usize, args: I) -> Result<usize, Error>
                             where I: iter::IntoIterator<Item = S>, S: AsRef<str> {
        let mut argptr = Vec::new();
        let mut argbuf = Vec::new();

        let mut argoff = off;
        for s in args {
            // push argv entry
            argptr.push(argoff);

            // push string
            let arg = s.as_ref().as_bytes();
            argbuf.extend_from_slice(arg);

            // 0-terminate it
            argbuf.push('\0' as u8);

            argoff += arg.len() + 1;
        }

        try!(self.mem.write(&argbuf, off));
        try!(self.mem.write(&argptr, argoff));
        Ok(argoff)
    }
}

pub fn init() {
    unsafe {
        CUR = Some(VPE::new_cur());
    }

    for _ in 0..FIRST_FREE_EP {
        VPE::cur().alloc_ep().unwrap();
    }
}
