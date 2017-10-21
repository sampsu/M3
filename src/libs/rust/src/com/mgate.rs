use cap;
use com::gate::Gate;
use errors::Error;
use dtu;
use kif;
use kif::INVALID_SEL;
use syscalls;
use util;
use vpe;

pub use kif::Perm;

#[derive(Debug)]
pub struct MemGate {
    gate: Gate,
}

pub struct MGateArgs {
    size: usize,
    addr: usize,
    perm: Perm,
    sel: cap::Selector,
    flags: cap::Flags,
}

impl MGateArgs {
    pub fn new(size: usize, perm: Perm) -> MGateArgs {
        MGateArgs {
            size: size,
            addr: !0,
            perm: perm,
            sel: INVALID_SEL,
            flags: cap::Flags::empty(),
        }
    }

    pub fn addr(mut self, addr: usize) -> Self {
        self.addr = addr;
        self
    }

    pub fn sel(mut self, sel: cap::Selector) -> Self {
        self.sel = sel;
        self
    }
}

impl MemGate {
    pub fn new(size: usize, perm: Perm) -> Result<Self, Error> {
        Self::new_with(MGateArgs::new(size, perm))
    }

    pub fn new_with(args: MGateArgs) -> Result<Self, Error> {
        let sel = if args.sel == INVALID_SEL {
            vpe::VPE::cur().alloc_cap()
        }
        else {
            args.sel
        };

        try!(syscalls::create_mgate(sel, args.addr, args.size, args.perm));
        Ok(MemGate {
            gate: Gate::new(sel, args.flags)
        })
    }

    pub fn new_bind(sel: cap::Selector) -> Self {
        MemGate {
            gate: Gate::new(sel, cap::Flags::KEEP_CAP)
        }
    }

    pub fn sel(&self) -> cap::Selector {
        self.gate.sel()
    }

    pub fn derive(&self, offset: usize, size: usize, perm: Perm) -> Result<Self, Error> {
        let sel = vpe::VPE::cur().alloc_cap();
        self.derive_with_sel(offset, size, perm, sel)
    }

    pub fn derive_with_sel(&self, offset: usize, size: usize,
                           perm: Perm, sel: cap::Selector) -> Result<Self, Error> {
        try!(syscalls::derive_mem(sel, self.sel(), offset, size, perm));
        Ok(MemGate {
            gate: Gate::new(sel, cap::Flags::empty())
        })
    }

    pub fn rebind(&mut self, sel: cap::Selector) -> Result<(), Error> {
        self.gate.rebind(sel)
    }

    pub fn read<T>(&self, data: &mut [T], off: usize) -> Result<(), Error> {
        self.read_bytes(data.as_mut_ptr() as *mut u8, data.len() * util::size_of::<T>(), off)
    }

    pub fn read_obj<T>(&self, data: *mut T, off: usize) -> Result<(), Error> {
        self.read_bytes(data as *mut u8, util::size_of::<T>(), off)
    }

    pub fn read_bytes(&self, mut data: *mut u8, mut size: usize, mut off: usize) -> Result<(), Error> {
        let ep = try!(self.gate.activate());

        loop {
            match dtu::DTU::read(ep, data, size, off, 0) {
                Ok(_)                           => return Ok(()),
                Err(e) if e == Error::VPEGone   => try!(self.forward_read(&mut data, &mut size, &mut off)),
                Err(e)                          => return Err(e),
            }
        }
    }

    pub fn write<T>(&self, data: &[T], off: usize) -> Result<(), Error> {
        self.write_bytes(data.as_ptr() as *const u8, data.len() * util::size_of::<T>(), off)
    }

    pub fn write_obj<T>(&self, obj: *const T, off: usize) -> Result<(), Error> {
        self.write_bytes(obj as *const u8, util::size_of::<T>(), off)
    }

    pub fn write_bytes(&self, mut data: *const u8, mut size: usize, mut off: usize) -> Result<(), Error> {
        let ep = try!(self.gate.activate());

        loop {
            match dtu::DTU::write(ep, data, size, off, 0) {
                Ok(_)                           => return Ok(()),
                Err(e) if e == Error::VPEGone   => try!(self.forward_write(&mut data, &mut size, &mut off)),
                Err(e)                          => return Err(e),
            }
        }
    }

    fn forward_read(&self, data: &mut *mut u8, size: &mut usize, off: &mut usize) -> Result<(), Error> {
        let amount = util::min(kif::syscalls::MAX_MSG_SIZE, *size);
        try!(syscalls::forward_read(
            self.sel(), util::slice_for_mut(*data, amount), *off,
            kif::syscalls::ForwardMemFlags::empty(), 0
        ));
        *data = unsafe { (*data).offset(amount as isize) };
        *off += amount;
        Ok(())
    }

    fn forward_write(&self, data: &mut *const u8, size: &mut usize, off: &mut usize) -> Result<(), Error> {
        let amount = util::min(kif::syscalls::MAX_MSG_SIZE, *size);
        try!(syscalls::forward_write(
            self.sel(), util::slice_for(*data, amount), *off,
            kif::syscalls::ForwardMemFlags::empty(), 0
        ));
        *data = unsafe { (*data).offset(amount as isize) };
        *off += amount;
        Ok(())
    }
}
