//! Jour 18 : ordonnanceur preemptive Round-Robin.
//!
//! Trois taches : MAIN (idle / executeur), A et B.
//! A et B bouclent sans jamais appeler `switch`. Seul IRQ0 les coupe,
//! toutes les QUANTUM ticks (10 ms * 10 = 100 ms).
//!
//! Appelé depuis le handler timer APRES `send_eoi(0)`, sans verrou VGA.

use core::ptr::{addr_of, addr_of_mut};
use core::sync::atomic::{AtomicU64, Ordering};
use crate::context::{self, ThreadContext};

pub const QUANTUM_TICKS: u64 = 10;
const STACK_SIZE: usize = 4096;
const MAX_TASKS: usize = 3;

static COUNTER_A: AtomicU64 = AtomicU64::new(0);
static COUNTER_B: AtomicU64 = AtomicU64::new(0);
static SWITCHES: AtomicU64 = AtomicU64::new(0);

static mut ENABLED: bool = false;
static mut SLICE: u64 = 0;
static mut CURRENT: usize = 0;
static mut COUNT: usize = 0;
static mut TASKS: [ThreadContext; MAX_TASKS] = [ThreadContext::empty(); MAX_TASKS];
static mut STACK_A: [u8; STACK_SIZE] = [0; STACK_SIZE];
static mut STACK_B: [u8; STACK_SIZE] = [0; STACK_SIZE];

extern "C" fn task_a() -> ! {
    loop {
        COUNTER_A.fetch_add(1, Ordering::Relaxed);
        core::hint::spin_loop();
    }
}

extern "C" fn task_b() -> ! {
    loop {
        COUNTER_B.fetch_add(1, Ordering::Relaxed);
        core::hint::spin_loop();
    }
}

pub fn counter_a() -> u64 {
    COUNTER_A.load(Ordering::Relaxed)
}

pub fn counter_b() -> u64 {
    COUNTER_B.load(Ordering::Relaxed)
}

pub fn switches() -> u64 {
    SWITCHES.load(Ordering::Relaxed)
}

/// Enregistre MAIN comme tache 0, cree A et B, arme l'ordonnanceur.
pub fn init() {
    unsafe {
        core::ptr::write(addr_of_mut!(COUNT), 1);
        core::ptr::write(addr_of_mut!(CURRENT), 0);
        core::ptr::write(addr_of_mut!(SLICE), 0);
        core::ptr::write(addr_of_mut!(ENABLED), false);

        ThreadContext::prepare(
            addr_of_mut!(TASKS).cast::<ThreadContext>().add(1),
            task_a,
            addr_of_mut!(STACK_A).cast(),
            STACK_SIZE,
        );
        ThreadContext::prepare(
            addr_of_mut!(TASKS).cast::<ThreadContext>().add(2),
            task_b,
            addr_of_mut!(STACK_B).cast(),
            STACK_SIZE,
        );
        core::ptr::write(addr_of_mut!(COUNT), 3);
        core::ptr::write(addr_of_mut!(ENABLED), true);
    }
}

/// Appele a chaque IRQ0, apres EOI. Ne prend aucun verrou.
pub fn on_timer_tick() {
    unsafe {
        if !core::ptr::read(addr_of!(ENABLED)) {
            return;
        }
        let slice = core::ptr::read(addr_of!(SLICE)) + 1;
        if slice < QUANTUM_TICKS {
            core::ptr::write(addr_of_mut!(SLICE), slice);
            return;
        }
        core::ptr::write(addr_of_mut!(SLICE), 0);
        schedule();
    }
}

unsafe fn schedule() {
    let n = core::ptr::read(addr_of!(COUNT));
    if n < 2 {
        return;
    }
    let old = core::ptr::read(addr_of!(CURRENT));
    let new = (old + 1) % n;
    if new == old {
        return;
    }
    core::ptr::write(addr_of_mut!(CURRENT), new);
    SWITCHES.fetch_add(1, Ordering::Relaxed);

    let base = addr_of_mut!(TASKS).cast::<ThreadContext>();
    context::switch(base.add(old), base.add(new));
}
