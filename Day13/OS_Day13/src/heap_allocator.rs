//! Jour 13 — Heap Allocator : Linked List Allocator

use crate::{serial, heap};

#[repr(C)]
struct BlockHeader {
    size: usize,
    free: bool,
    next: *mut BlockHeader,
}

const HEADER_SIZE: usize = core::mem::size_of::<BlockHeader>();
const ALIGN: usize = 8;

fn align_up(size: usize, align: usize) -> usize {
    (size + align - 1) & !(align - 1)
}

pub struct LinkedListAllocator {
    head: *mut BlockHeader,
    initialized: bool,
}

impl LinkedListAllocator {
    pub const fn new() -> Self {
        LinkedListAllocator { head: core::ptr::null_mut(), initialized: false }
    }

    pub fn init(&mut self) {
        let heap_start = heap::HEAP_START as *mut BlockHeader;
        let heap_size  = heap::HEAP_SIZE as usize;
        unsafe {
            (*heap_start).size = heap_size - HEADER_SIZE;
            (*heap_start).free = true;
            (*heap_start).next = core::ptr::null_mut();
        }
        self.head = heap_start;
        self.initialized = true;
        serial::println("  Linked list allocateur initialise");
    }

    pub fn alloc(&mut self, size: usize) -> Option<*mut u8> {
        if !self.initialized { return None; }
        let size = align_up(size.max(1), ALIGN);
        let mut current = self.head;
        while !current.is_null() {
            unsafe {
                if (*current).free && (*current).size >= size {
                    let remaining = (*current).size - size;
                    if remaining > HEADER_SIZE + ALIGN {
                        let new_block = (current as *mut u8).add(HEADER_SIZE + size) as *mut BlockHeader;
                        (*new_block).size = remaining - HEADER_SIZE;
                        (*new_block).free = true;
                        (*new_block).next = (*current).next;
                        (*current).size = size;
                        (*current).next = new_block;
                    }
                    (*current).free = false;
                    return Some((current as *mut u8).add(HEADER_SIZE));
                }
                current = (*current).next;
            }
        }
        None
    }

    pub fn free(&mut self, ptr: *mut u8) -> bool {
        if ptr.is_null() { return false; }
        let header = unsafe { (ptr.sub(HEADER_SIZE)) as *mut BlockHeader };
        unsafe {
            if (*header).free {
                serial::println("  ERREUR : double-free detecte !");
                return false;
            }
            (*header).free = true;
        }
        self.coalesce();
        true
    }

    fn coalesce(&mut self) {
        let mut current = self.head;
        while !current.is_null() {
            unsafe {
                let next = (*current).next;
                if !next.is_null() && (*current).free && (*next).free {
                    (*current).size += HEADER_SIZE + (*next).size;
                    (*current).next  = (*next).next;
                } else {
                    current = (*current).next;
                }
            }
        }
    }

    pub fn print_map(&self) {
        serial::println("\n=== Linked List Allocateur ===");
        let mut current = self.head;
        let mut i = 0usize;
        while !current.is_null() {
            unsafe {
                serial::print("  Bloc "); print_num(i);
                serial::print(" @ "); print_hex(current as u64);
                serial::print(" | size="); print_num((*current).size);
                serial::print(" | ");
                if (*current).free { serial::println("LIBRE"); } else { serial::println("UTILISE"); }
                current = (*current).next;
                i += 1;
            }
        }
        serial::println("=== Fin linked list ===\n");
    }

    pub fn stats(&self) -> (usize, usize, usize) {
        let mut free_b = 0usize;
        let mut used_b = 0usize;
        let mut count  = 0usize;
        let mut current = self.head;
        while !current.is_null() {
            unsafe {
                count += 1;
                if (*current).free { free_b += (*current).size; } else { used_b += (*current).size; }
                current = (*current).next;
            }
        }
        (free_b, used_b, count)
    }
}

pub static mut HEAP_ALLOC: LinkedListAllocator = LinkedListAllocator::new();

pub fn init() {
    unsafe { (*core::ptr::addr_of_mut!(HEAP_ALLOC)).init(); }
}

pub fn alloc(size: usize) -> Option<*mut u8> {
    unsafe { (*core::ptr::addr_of_mut!(HEAP_ALLOC)).alloc(size) }
}

pub fn free(ptr: *mut u8) -> bool {
    unsafe { (*core::ptr::addr_of_mut!(HEAP_ALLOC)).free(ptr) }
}

fn print_hex(v: u64) {
    serial::print("0x");
    let mut buf = [0u8; 16];
    let mut i = 16usize;
    let mut n = v;
    if n == 0 { serial::print("0"); return; }
    while n > 0 && i > 0 {
        i -= 1;
        let nibble = (n & 0xF) as u8;
        buf[i] = if nibble < 10 { b'0' + nibble } else { b'a' + nibble - 10 };
        n >>= 4;
    }
    for &b in &buf[i..] { serial::print(core::str::from_utf8(&[b]).unwrap_or("?")); }
}

fn print_num(n: usize) {
    if n == 0 { serial::print("0"); return; }
    let mut buf = [0u8; 20];
    let mut i = 20usize;
    let mut v = n;
    while v > 0 && i > 0 { i -= 1; buf[i] = b'0' + (v % 10) as u8; v /= 10; }
    for &b in &buf[i..] { serial::print(core::str::from_utf8(&[b]).unwrap_or("?")); }
}
