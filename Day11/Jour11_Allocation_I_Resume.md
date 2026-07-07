# Jour 11 - Allocation I : Frame Allocator

---

## Introduction

On a un mapping fixe de 32Mo créé dans `stage2.asm`. Pour allouer dynamiquement (heap, nouveaux processus), il faut un **allocateur de frames** qui sait quelles pages physiques de 2Mo sont libres et peut les distribuer ou les récupérer.

---

## Les 3 états d'une frame

```rust
pub enum FrameState {
    Free,      // disponible
    Used,      // allouée
    Reserved,  // kernel/tables - jamais libérable
}
```

## Répartition initiale

```
Frame 0  [0x000000  - 0x1fffff ] : RESERVE  (zone basse BIOS/VGA)
Frame 1  [0x200000  - 0x3fffff ] : RESERVE  (kernel)
Frame 2  [0x400000  - 0x5fffff ] : RESERVE  (tables de pages)
Frame 3+ [0x600000+ - ...      ] : LIBRE    (13 frames disponibles)
```

---

## Code crucial

```rust
pub const FRAME_SIZE:  u64   = 2 * 1024 * 1024;
pub const FRAME_COUNT: usize = 16;

pub struct FrameAllocator {
    frames: [FrameState; FRAME_COUNT],
    free_count: usize,
}

// Allocation first-fit
pub fn alloc(&mut self) -> Option<u64> {
    for i in 0..FRAME_COUNT {
        if self.frames[i] == FrameState::Free {
            self.frames[i] = FrameState::Used;
            self.free_count -= 1;
            return Some(MAPPING_START + (i as u64 * FRAME_SIZE));
        }
    }
    None  // mémoire épuisée → Option force la gestion d'erreur
}

// Libération avec vérifications
pub fn free(&mut self, phys_addr: u64) -> bool {
    let index = ((phys_addr - MAPPING_START) / FRAME_SIZE) as usize;
    if index >= FRAME_COUNT           { return false; } // hors mapping
    if self.frames[index] == Reserved { return false; } // zone réservée
    if self.frames[index] == Used {
        self.frames[index] = FrameState::Free;
        self.free_count += 1;
        return true;
    }
    false // double free détecté
}

// Accès au static mutable - obligatoire Rust 2024
pub static mut ALLOCATOR: FrameAllocator = FrameAllocator::new();
pub fn alloc_frame() -> Option<u64> {
    unsafe { (*core::ptr::addr_of_mut!(ALLOCATOR)).alloc() }
}
```

---

## Arborescence

![Structure des fichiers et dossiers](./arborescence.png)

```
OS_Day11/src/
├── main.rs             ← 4 tests : alloc, free, realloc, épuisement
├── frame_allocator.rs  ← NOUVEAU
└── ...
```

---

## Résultat obtenu

![Rendu - Allocations et libérations réussies](./rendu1.png)

![Rendu - Test d'épuisement mémoire géré](./rendu2.png)

```
Frames : 13 libres / 16 total

Test 1 : allocation x 3
  -> 0x600000, 0x800000, 0xa00000 (first-fit)

Test 2 : liberation 0x600000
  -> Frame remise à Free

Test 3 : reallocation
  -> 0x600000 réutilisée immédiatement ✅

Test 4 : épuisement
  -> None retourné - pas de crash ✅
  -> Frames : 0 libres / 16 total
```

---

## Angle Cyber

| Risque | Protection |
|---|---|
| Double free | `free()` retourne `false` si déjà `Free` |
| Free zone réservée | Refusé si état `Reserved` |
| Épuisement mémoire | `Option<u64>` - `None` sans crash |
| Hors mapping | Vérifie `index >= FRAME_COUNT` |

> En C, `malloc()` retourne `NULL` souvent non vérifié -> crash ou exploit. En Rust, `Option<u64>` **force** la gestion du cas d'échec à la compilation. C'est la différence fondamentale entre les deux langages pour la sécurité mémoire.
