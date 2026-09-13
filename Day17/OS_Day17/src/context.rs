//! Jour 17 : contexte CPU et commutation cooperative.
//!
//! `switch_context` sauvegarde les registres callee-saved (convention SysV),
//! RSP, RIP et RFLAGS, puis reprend l'autre tache. C'est assez pour relancer
//! une fonction Rust. Le Jour 18 ajoutera l'etat caller-saved et FXSAVE,
//! indispensables quand IRQ0 preemptre au milieu d'une instruction SSE.
//!
//! Rust 2024 interdit `&` / `&mut` sur un `static mut`. Tout acces passe
//! par `addr_of!` / `addr_of_mut!` et des pointeurs bruts.

use core::ptr::{addr_of, addr_of_mut};
use crate::serial;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ThreadContext {
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

    /// Prepare une tache neuve : pile alignee, RIP = point d'entree.
    ///
    /// `rsp` est pose a `top - 8` pour respecter SysV : a l'entree d'une
    /// fonction, RSP % 16 == 8 (comme apres un `call`).
    pub unsafe fn prepare(this: *mut Self, entry: extern "C" fn() -> !, stack: *mut u8, stack_len: usize) {
        this.write(Self::empty());
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

const STACK_SIZE: usize = 4096;

static mut MAIN_CTX: ThreadContext = ThreadContext::empty();
static mut TASK_A_CTX: ThreadContext = ThreadContext::empty();
static mut TASK_B_CTX: ThreadContext = ThreadContext::empty();
static mut STACK_A: [u8; STACK_SIZE] = [0; STACK_SIZE];
static mut STACK_B: [u8; STACK_SIZE] = [0; STACK_SIZE];
static mut SWITCH_COUNT: u64 = 0;
static mut SEQUENCE: [u8; 8] = [0; 8];
static mut SEQ_LEN: usize = 0;

fn record(tag: u8) {
    unsafe {
        let n = core::ptr::read(addr_of!(SEQ_LEN));
        if n < 8 {
            core::ptr::write(addr_of_mut!(SEQUENCE).cast::<u8>().add(n), tag);
            core::ptr::write(addr_of_mut!(SEQ_LEN), n + 1);
        }
        let count = core::ptr::read(addr_of!(SWITCH_COUNT));
        core::ptr::write(addr_of_mut!(SWITCH_COUNT), count + 1);
    }
}

extern "C" fn task_a() -> ! {
    serial::println("  [A] premiere execution, pile A");
    record(b'A');
    unsafe { switch(addr_of_mut!(TASK_A_CTX), addr_of!(TASK_B_CTX)) };

    serial::println("  [A] retour apres B, registres et pile restaures");
    record(b'A');
    unsafe { switch(addr_of_mut!(TASK_A_CTX), addr_of!(MAIN_CTX)) };

    loop {
        unsafe { core::arch::asm!("hlt") }
    }
}

extern "C" fn task_b() -> ! {
    serial::println("  [B] execution sur une autre pile");
    record(b'B');
    unsafe { switch(addr_of_mut!(TASK_B_CTX), addr_of!(TASK_A_CTX)) };

    loop {
        unsafe { core::arch::asm!("hlt") }
    }
}

/// Enchaine main -> A -> B -> A -> main. Renvoie le nombre de commutations.
pub fn demo() -> u64 {
    unsafe {
        core::ptr::write(addr_of_mut!(SWITCH_COUNT), 0);
        core::ptr::write(addr_of_mut!(SEQ_LEN), 0);

        ThreadContext::prepare(
            addr_of_mut!(TASK_A_CTX),
            task_a,
            addr_of_mut!(STACK_A).cast(),
            STACK_SIZE,
        );
        ThreadContext::prepare(
            addr_of_mut!(TASK_B_CTX),
            task_b,
            addr_of_mut!(STACK_B).cast(),
            STACK_SIZE,
        );

        serial::println("  main -> A");
        switch(addr_of_mut!(MAIN_CTX), addr_of!(TASK_A_CTX));
        serial::println("  retour dans main");

        core::ptr::read(addr_of!(SWITCH_COUNT))
    }
}

pub fn sequence() -> ([u8; 8], usize) {
    unsafe {
        let n = core::ptr::read(addr_of!(SEQ_LEN));
        let mut buf = [0u8; 8];
        core::ptr::copy_nonoverlapping(addr_of!(SEQUENCE).cast::<u8>(), buf.as_mut_ptr(), n.min(8));
        (buf, n)
    }
}
