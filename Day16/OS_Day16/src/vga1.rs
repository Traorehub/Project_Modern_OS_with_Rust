// src/vga1.rs – spinlock maison (copy from Day5)

use core::sync::atomic::{AtomicBool, Ordering};
use core::cell::UnsafeCell;

/// Simple spinlock implémenté avec `AtomicBool`.
/// Le lock est libéré automatiquement quand le `MutexGuard` est drop.
pub struct Mutex<T> {
    lock: AtomicBool,
    data: UnsafeCell<T>,
}

unsafe impl<T> Sync for Mutex<T> {}

impl<T> Mutex<T> {
    pub const fn new(data: T) -> Self {
        Self {
            lock: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    /// Bloque jusqu'à obtenir le lock, puis retourne un guard.
    #[inline]
    pub fn lock(&self) -> MutexGuard<T> {
        // spin‑loop jusqu'à ce que le lock soit libéré
        while self
            .lock
            .swap(true, Ordering::Acquire)
        {
            core::hint::spin_loop();
        }
        MutexGuard { lock: &self.lock, data: self.data.get() }
    }
}

pub struct MutexGuard<'a, T> {
    lock: &'a AtomicBool,
    data: *mut T,
}

impl<'a, T> core::ops::Deref for MutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target { unsafe { &*self.data } }
}

impl<'a, T> core::ops::DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target { unsafe { &mut *self.data } }
}

impl<'a, T> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        self.lock.store(false, Ordering::Release);
    }
}
