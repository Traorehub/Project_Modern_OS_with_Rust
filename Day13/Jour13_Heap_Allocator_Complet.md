# Jour 13 - Heap Allocator : Linked List Allocator

---

## Introduction

Au Jour 12, on a mappé un heap de 8Mo - une zone mémoire virtuelle accessible. Mais c'est un bloc unique : impossible d'allouer dynamiquement des morceaux de taille variable. Au Jour 13, on implémente un **véritable allocateur** au-dessus de ce heap : un **linked list allocator** qui gère des blocs de taille variable, les divise, les fusionne, et détecte les erreurs classiques (double-free, use-after-free).

---

## Concepts fondamentaux

### Structure d'un bloc

```
+----------------+-------------------------+
| BlockHeader    |   Zone de données       |
| (24 octets)    |   (taille variable)     |
+----------------+-------------------------+
      ^                    ^
   métadonnées      pointeur retourné
                     à l'utilisateur
```

```rust
struct BlockHeader {
    size: usize,           // taille de la zone de données
    free: bool,            // libre ou utilisé
    next: *mut BlockHeader, // bloc suivant dans la liste chaînée
}
```

### Le principe linked list

```
HEAP_START
    |
[Header|Data] -> [Header|Data] -> [Header|Data] -> NULL
   Bloc 0          Bloc 1           Bloc 2
```

Chaque bloc connaît le suivant via `next`. Pas besoin de tableau ni d'index - la liste s'étend dynamiquement dans la mémoire du heap elle-même.

### Allocation - first-fit avec division

```
1 grand bloc libre (8Mo)
    | alloc(64)
[UTILISE 64o] -> [LIBRE reste]
    | alloc(128)
[UTILISE 64o] -> [UTILISE 128o] -> [LIBRE reste]
```

Si le bloc trouvé est **beaucoup plus grand** que nécessaire, on le **divise** en deux : la partie utilisée et le reste qui redevient un bloc libre.

```rust
let remaining = (*current).size - size;
if remaining > HEADER_SIZE + ALIGN {
    // Diviser : créer un nouveau bloc avec le reste
    let new_block = (current as *mut u8).add(HEADER_SIZE + size) as *mut BlockHeader;
    (*new_block).size = remaining - HEADER_SIZE;
    (*new_block).free = true;
    (*new_block).next = (*current).next;
    (*current).size = size;
    (*current).next = new_block;
}
```

### Libération - coalescence (fusion)

Sans fusion, libérer puis réallouer crée de la **fragmentation** : plein de petits blocs libres trop petits individuellement pour de grosses allocations.

```
Avant coalescence :
[LIBRE 64o] -> [LIBRE 128o] -> [UTILISE 256o]

Après coalescence :
[LIBRE 216o] -> [UTILISE 256o]
       ^
  64 + 24(header) + 128 = 216
```

```rust
fn coalesce(&mut self) {
    let mut current = self.head;
    while !current.is_null() {
        let next = (*current).next;
        if !next.is_null() && (*current).free && (*next).free {
            (*current).size += HEADER_SIZE + (*next).size;
            (*current).next  = (*next).next;
            // Ne pas avancer - revérifier le même bloc fusionné
        } else {
            current = (*current).next;
        }
    }
}
```

---

## Détection des erreurs classiques

### Double-free

```rust
pub fn free(&mut self, ptr: *mut u8) -> bool {
    let header = unsafe { (ptr.sub(HEADER_SIZE)) as *mut BlockHeader };
    unsafe {
        if (*header).free {
            serial::println("  ERREUR : double-free detecte !");
            return false; // refuse - bloc déjà libre
        }
        (*header).free = true;
    }
    self.coalesce();
    true
}
```

En retrouvant l'en-tête juste avant le pointeur (`ptr.sub(HEADER_SIZE)`), on vérifie son état avant de le libérer.

### Use-After-Free (UAF) - démonstration volontaire

```rust
let p4 = heap_allocator::alloc(64);
if let Some(ptr) = p4 {
    unsafe { core::ptr::write_volatile(ptr as *mut u64, 0x1111_2222_3333_4444); }
    heap_allocator::free(ptr);
    // Lecture APRÈS free - UAF classique
    unsafe {
        let val = core::ptr::read_volatile(ptr as *const u64);
        // val == 0x1111_2222_3333_4444 encore lisible !
    }
}
```

**Ce test prouve volontairement le danger** : en `unsafe`, Rust permet exactement la même erreur qu'en C - lire une zone après l'avoir libérée. C'est pour ça que le code "safe" Rust (sans `unsafe`) interdit ce pattern via l'ownership.

---

## Arborescence

![Structure des fichiers et dossiers du projet](./arborescence.png)

```
OS_Day13/
└── src/
    ├── main.rs            <- "Jour 13 - Heap Allocator", 5 tests
    ├── heap_allocator.rs  <- NOUVEAU : LinkedListAllocator
    ├── heap.rs            <- map_heap() du Jour 12
    ├── frame_allocator.rs
    ├── paging.rs
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
  -display none \
  -no-reboot -no-shutdown
```

---

## Résultat obtenu

![Rendu - sortie série complète du kernel : allocations, coalescence, double-free et UAF](./rendu.png)

### État initial - un seul bloc

```
Bloc 0 @ 0x4000000 | size=8388584 | LIBRE
```

Au démarrage, le heap entier (8Mo moins l'en-tête de 24 octets, soit `8388608 - 24 = 8388584`) est représenté par un **unique bloc libre**. C'est la preuve que l'allocateur démarre proprement sans aucune fragmentation initiale.

### Test 1+2 - allocation et calcul d'offset

```
p1 = 0x4000018 (64o)
p2 = 0x4000070 (128o)
p3 = 0x4000108 (256o)
p1 -> ecriture/lecture : OK
```

Vérification mathématique :
```
p1 = HEAP_START + HEADER_SIZE = 0x4000000 + 24 = 0x4000018 OK
p2 = p1 + size(64) + HEADER_SIZE(24) = 0x4000070 OK
```

Chaque adresse retournée correspond exactement au calcul attendu. Le test d'écriture/lecture confirme que la zone est réellement accessible en mémoire physique mappée.

### Après allocations - division des blocs

```
Bloc 0 @ 0x4000000 | size=64    | UTILISE
Bloc 1 @ 0x4000058 | size=128   | UTILISE
Bloc 2 @ 0x40000f0 | size=256   | UTILISE
Bloc 3 @ 0x4000208 | size=8388064 | LIBRE
```

Le bloc unique initial de 8Mo a été **divisé en 4 morceaux** : trois blocs `UTILISE` et un grand bloc `LIBRE` restant. La logique de découpage fonctionne - l'allocateur ne gaspille pas la mémoire.

### Test 3 - coalescence prouvée par le calcul

```
Avant : Bloc 0 (64, UTILISE) + Bloc 1 (128, UTILISE)
free(p1) + free(p2) ->
Bloc 0 @ 0x4000000 | size=216 | LIBRE
```

Vérification : `64 + 24(header de bloc1) + 128 = 216` - ce calcul exact prouve que la fusion a réellement eu lieu en mémoire.

Deux blocs adjacents libérés séparément ont été **fusionnés automatiquement** en un seul bloc de taille `216`. Sans coalescence, ces deux petits blocs resteraient inutilisables pour une allocation de 200 octets par exemple.

### Test 4 - double-free détecté

```
ERREUR : double-free detecte !
Double-free detecte et refuse OK
```

Un deuxième `free()` sur le même bloc est intercepté avant toute corruption - l'allocateur vérifie l'état `free` du bloc avant d'agir.

### Test 5 - use-after-free démontré

```
Valeur apres free (UAF) : 0x1111222233334444
(dangereux en C, interdit en Rust safe)
```

Ce test est **volontairement dangereux** pour illustrer le risque. On écrit une valeur, on libère le bloc, puis on relit - la valeur est toujours là. En production, cette zone pourrait avoir été réallouée à un autre composant. Ce test n'est possible qu'en `unsafe` - le Rust safe interdit ce pattern via l'ownership.

### Stats finales

```
Blocs : 3
Libre : 8388280 octets
Utilise : 256 octets   <- uniquement p3 (256o) reste alloué
```

Seul `p3` (256 octets) reste effectivement alloué. p1 et p2 ont été libérés et fusionnés dans le grand bloc libre.

---

## Résumé - Angle Cyber

| Mécanisme | Faille prévenue / démontrée |
|---|---|
| `BlockHeader` avant chaque allocation | Permet de retrouver les métadonnées sans table externe |
| Vérification `if (*header).free` dans `free()` | Détecte le double-free avant corruption |
| Coalescence | Réduit la fragmentation - évite l'épuisement mémoire prématuré |
| Test UAF volontaire en `unsafe` | Démontre le danger réel - Rust safe l'empêche via ownership |
| `align_up()` à 8 octets | Évite les problèmes d'alignement mémoire sur x86_64 |

> Notre linked list allocator reproduit les vulnérabilités classiques des allocateurs C (`dlmalloc`, `ptmalloc`) : si le `BlockHeader` est corrompu par un buffer overflow adjacent, l'attaquant peut forger un faux pointeur `next` et rediriger une écriture vers une adresse arbitraire. C'est le mécanisme **heap overflow -> arbitrary write** présent dans de nombreux CVEs. Le Rust safe empêche ce scénario via l'ownership, mais notre allocateur utilise nécessairement `unsafe` - il doit donc être audité avec la même rigueur qu'un allocateur C.
