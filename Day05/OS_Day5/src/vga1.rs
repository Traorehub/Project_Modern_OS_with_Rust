//! Mutex Spinlock - Jour 5 (vga1)
#![allow(dead_code)]

use core::sync::atomic::{AtomicBool, Ordering};
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};

// ─── Simple Spinlock Mutex ───────────────────────────────────────────────────
pub struct Mutex<T> {
    lock: AtomicBool,
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Sync for Mutex<T> {}

impl<T> Mutex<T> {
    pub const fn new(data: T) -> Self {
        Mutex {
            lock: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    pub fn lock(&self) -> MutexGuard<T> {
        // Tente d'obtenir le lock (spin loop)
        while self.lock.swap(true, Ordering::Acquire) {
            core::hint::spin_loop();
        }
        MutexGuard { lock: &self.lock, data: unsafe { &mut *self.data.get() } }
    }
}

pub struct MutexGuard<'a, T: 'a> {
    lock: &'a AtomicBool,
    data: &'a mut T,
}

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T { self.data }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T { self.data }
}

impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.store(false, Ordering::Release);
    }
}
