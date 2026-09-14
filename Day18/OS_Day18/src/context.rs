//! Jour 17/18 : contexte CPU.
//!
//! Jour 17 : callee-saved suffisait (switch volontaire).
//! Jour 18 : IRQ0 peut preemptre au milieu du SSE, donc FXSAVE/FXRSTOR
//! (512 octets, buffer aligne 16).
//!
//! Rust 2024 : pas de `&` / `&mut` sur un `static mut`.

use core::ptr::addr_of_mut;

#[repr(C, align(16))]
#[derive(Clone, Copy)]
pub struct ThreadContext {
    pub fx: [u8; 512],
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub rbx: u64,
    pub rbp: u64,
    pub rsp: u64,
    pub rip: u64,
    pub rflags: u64,
}

impl ThreadContext {
    pub const fn empty() -> Self {
        ThreadContext {
            fx: [0; 512],
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            rbx: 0,
            rbp: 0,
            rsp: 0,
            rip: 0,
            rflags: 0,
        }
    }

    /// Image FX valide pour `fxrstor` : FCW = 0x037F, MXCSR = 0x1F80.
    unsafe fn init_fx(this: *mut Self) {
        let fx = addr_of_mut!((*this).fx).cast::<u8>();
        core::ptr::write_bytes(fx, 0, 512);
        core::ptr::write(fx.cast::<u16>(), 0x037F);
        core::ptr::write(fx.add(24).cast::<u32>(), 0x1F80);
    }

    pub unsafe fn prepare(this: *mut Self, entry: extern "C" fn() -> !, stack: *mut u8, stack_len: usize) {
        this.write(Self::empty());
        Self::init_fx(this);
        let top = (stack as u64 + stack_len as u64) & !0xF;
        (*this).rsp = top.saturating_sub(8);
        (*this).rip = entry as u64;
        (*this).rflags = 0x202;
    }
}

unsafe extern "C" {
    fn switch_context(old: *mut ThreadContext, new: *const ThreadContext);
}

pub unsafe fn switch(old: *mut ThreadContext, new: *const ThreadContext) {
    unsafe { switch_context(old, new) };
}
