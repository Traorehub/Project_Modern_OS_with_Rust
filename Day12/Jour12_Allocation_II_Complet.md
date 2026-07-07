# Jour 12 - Allocation II : Mapping du Heap

---

## Introduction

Au Jour 11, on a créé un allocateur de frames physiques. Au Jour 12, on l'utilise pour **mapper dynamiquement un heap** - une zone mémoire virtuelle dédiée à l'allocation dynamique. On alloue des frames physiques via l'allocateur, on les insère dans les tables de pages existantes, puis on vérifie que le mapping est réel en écrivant et lisant des données - et en prouvant que les adresses hors heap déclenchent bien un Page Fault.

---

## Concepts fondamentaux

### Allocation statique vs dynamique

```
Allocation statique :
static mut BUFFER: [u8; 1024] = [0; 1024];
→ Taille connue à la compilation
→ Placée dans .data ou .bss
→ Existe pendant toute la durée du programme
→ Pas de fragmentation possible

Allocation dynamique (heap) :
let ptr = alloc(1024); // taille choisie à l'exécution
→ Taille inconnue à la compilation
→ Placée dans une zone dédiée (heap)
→ Peut être libérée et réutilisée
→ Fragmentation possible
```

### Pourquoi mapper le heap ?

Le heap n'existe pas "naturellement" - c'est une convention. Il faut :
1. **Choisir** une plage d'adresses virtuelles (ex: `0x4000000 - 0x47fffff`)
2. **Allouer** des frames physiques via le frame allocator
3. **Mapper** ces frames dans les tables de pages
4. **Vérifier** que les adresses hors heap ne sont pas accessibles

---

## Architecture du heap

```
Espace virtuel :
0x000000  - 0x1fffff  : zone basse (BIOS, VGA)
0x200000  - 0x3fffff  : kernel (.text, .data)
0x400000  - 0x5fffff  : tables de pages
...
0x4000000 - 0x47fffff : HEAP (8Mo = 4 frames × 2Mo)  ← Jour 12
...
0x8000000+            : non mappé → Page Fault si accès
```

### Calcul des index PD

```
0x4000000 / 2Mo = 0x4000000 / 0x200000 = 32

→ PD[32] = 0x4000000 - 0x41fffff (frame 1)
→ PD[33] = 0x4200000 - 0x43fffff (frame 2)
→ PD[34] = 0x4400000 - 0x45fffff (frame 3)
→ PD[35] = 0x4600000 - 0x47fffff (frame 4)
```

---

## Implémentation — `src/heap.rs`

### Mapping

```rust
pub const HEAP_START: u64 = 0x4000000;
pub const HEAP_SIZE:  u64 = 4 * 2 * 1024 * 1024; // 8Mo
pub const HEAP_END:   u64 = HEAP_START + HEAP_SIZE;

pub fn map_heap() -> bool {
    let heap_pd_start = (HEAP_START / (2 * 1024 * 1024)) as usize; // = 32

    // Récupérer l'adresse de la PD depuis CR3 → PML4 → PDPT → PD
    let pd_addr = /* ... lecture des tables existantes ... */;

    for i in 0..4usize {
        let pd_index = heap_pd_start + i;           // 32, 33, 34, 35
        let pd_entry_addr = pd_addr + (pd_index * 8) as u64;

        // Allouer une frame physique
        let phys_addr = frame_allocator::alloc_frame()?;

        // Créer l'entrée PD : Present + Write + Huge (0x83)
        let pd_entry = phys_addr | 0x83;

        // Écrire dans la table de pages + invalider TLB
        unsafe {
            core::ptr::write_volatile(pd_entry_addr as *mut u64, pd_entry);
            core::arch::asm!("invlpg [{addr}]",
                addr = in(reg) HEAP_START + (i as u64 * 2 * 1024 * 1024));
        }
    }
    true
}
```

### Tests de validation

```rust
pub fn test_heap() {
    // Test 1 : écriture/lecture début heap
    unsafe {
        let ptr = HEAP_START as *mut u64;
        core::ptr::write_volatile(ptr, 0xDEADBEEF_CAFEBABE);
        assert!(core::ptr::read_volatile(ptr) == 0xDEADBEEF_CAFEBABE);
    }

    // Test 2 : écriture/lecture fin heap
    unsafe {
        let ptr = (HEAP_END - 8) as *mut u64;
        core::ptr::write_volatile(ptr, 0x1234_5678_9ABC_DEF0);
        assert!(core::ptr::read_volatile(ptr) == 0x1234_5678_9ABC_DEF0);
    }

    // Test 3 : remplissage 1Ko
    unsafe {
        let base = HEAP_START as *mut u64;
        for i in 0..128 { core::ptr::write_volatile(base.add(i), i as u64); }
        for i in 0..128 { assert!(core::ptr::read_volatile(base.add(i)) == i as u64); }
    }
}
```

### Test de validation — preuve que le mapping est réel

```rust
// Accès à une adresse hors heap -> Page Fault -> prouve l'isolation
serial::println("Test validation : acces hors heap...");
unsafe {
    let bad_ptr = 0x8000000 as *mut u64; // hors heap, non mappé
    core::ptr::write_volatile(bad_ptr, 42); // -> Page Fault déclenché
}
// Cette ligne ne s'exécute jamais -> Page Fault intercepté par l'IDT
```

---

## Arborescence

![Structure des fichiers et dossiers](./arborescence.png)

```
OS_Day12/
└── src/
    ├── main.rs          ← "Jour 12 - Allocation II"
    ├── heap.rs          ← NOUVEAU : map_heap() + test_heap()
    ├── frame_allocator.rs
    ├── paging.rs
    ├── gdt.rs
    ├── idt.rs
    └── ...
```

---

## Commandes

```bash
cargo build -Z build-std=core --target x86_64-unknown-none

objcopy -O binary \
  --only-section=.text \
  --only-section=.rodata \
  --only-section=.data \
  target/x86_64-unknown-none/debug/kernel kernel.bin

dd if=/dev/zero   of=os.img bs=512 count=8192
dd if=boot.bin    of=os.img conv=notrunc
dd if=stage2.bin  of=os.img bs=512 seek=1 conv=notrunc
dd if=kernel.bin  of=os.img bs=512 seek=2 conv=notrunc

qemu-system-x86_64 \
  -drive format=raw,file=os.img \
  -serial stdio \
  -display sdl \
  -no-reboot -no-shutdown
```

---

## Résultat obtenu

![Rendu - Mapping du heap et tests de lecture/écriture réussis](./rendu_sans%20_maping_corrompu.png)

![Rendu - Page Fault et Double Fault interceptés lors d'un accès hors heap](./rendu_lors_du_test_de_mapping_corronpu.png)

```
Initialisation allocateur...
  Frames : 13 libres / 16 total

=== Mapping du heap ===
  Zone virtuelle : 0x4000000 - 0x47fffff
  Mappe PD[32] virt=0x4000000 -> phys=0x600000
  Mappe PD[33] virt=0x4200000 -> phys=0x800000
  Mappe PD[34] virt=0x4400000 -> phys=0xa00000
  Mappe PD[35] virt=0x4600000 -> phys=0xc00000
  4 frames mappees pour le heap

Frames apres mapping heap :
  Frames : 9 libres / 16 total    ← 4 frames consommées ✅

=== Test du heap ===
  Test ecriture 0x4000000  : OK   ← début heap
  Test ecriture 0x47ffff8  : OK   ← fin heap
  Remplissage 1Ko depuis 0x4000000 : OK

=== Statique vs Dynamique ===
  Statique  : taille fixe a la compilation
  Heap      : taille variable a l'execution
  Notre heap: 8Mo mappe dynamiquement

Test validation : acces hors heap...
[PAGE FAULT] Corruption stack -> Double Fault...
[DOUBLE FAULT] IST OK!             <- preuve que 0x8000000 n'est pas mappé ✅
```

### Interprétation

| Observation | Ce que ça prouve |
|---|---|
| `Frames : 13 -> 9` | 4 frames physiques réellement allouées |
| `PD[32..35] mappés` | Tables de pages modifiées en RAM |
| `Test ecriture : OK` | Les adresses virtuelles heap sont accessibles |
| `Page Fault sur 0x8000000` | L'isolation fonctionne - hors heap = non mappé |

---

## Résumé - Angle Cyber

### Allocation statique vs dynamique en sécurité

| | Statique | Dynamique (heap) |
|---|---|---|
| Taille | Fixe à la compilation | Variable à l'exécution |
| Localisation | `.data` / `.bss` | Zone dédiée mappée |
| Vulnérabilités | Stack overflow, BSS overflow | Heap overflow, use-after-free, double-free |
| Protection Rust | Vérification de bornes | `Option<T>`, ownership |

### L'isolation mémoire prouvée

```
0x4000000 - 0x47fffff : MAPPÉ -> accès OK
0x8000000+            : NON MAPPÉ -> Page Fault immédiat

-> Le CPU applique cette règle matériellement
-> Impossible de contourner sans modifier les tables de pages
-> Modifier les tables de pages nécessite Ring 0
```

### Vecteurs d'attaque sur le heap

```
Heap overflow :
Écrire au-delà d'un buffer alloué -> écraser les métadonnées
-> Corruption de l'allocateur -> contrôle du flux d'exécution

Use-after-free :
Utiliser un pointeur après free()
-> La zone peut être réallouée -> données d'un autre objet

Double-free :
Libérer deux fois la même zone
-> Corruption de l'allocateur -> état indéfini

Notre protection :
-> Rust ownership : impossible d'avoir deux propriétaires
-> Page Fault si accès hors mapping
-> frame_allocator::free() détecte les double-free
```

> Le heap est la zone la plus attaquée d'un OS moderne. Chrome, Firefox, Linux kernel - la grande majorité des CVEs exploitables sont des heap exploits. Notre implémentation est basique mais les principes sont les mêmes : isoler la zone, vérifier les bornes, détecter les doubles libérations.
