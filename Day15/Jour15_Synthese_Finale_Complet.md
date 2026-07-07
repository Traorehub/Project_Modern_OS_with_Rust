# Jour 15 — Synthèse finale : revue rétrospective du projet

---

## Introduction

Après 14 jours de construction incrémentale, le Jour 15 consolide tout dans **un seul kernel fonctionnel** (`Day15/OS_Day15/`). L'objectif n'est pas d'apprendre une nouvelle API matérielle, mais de répondre à une question simple :

> Est-ce que tout ce qu'on a construit tient debout quand on l'exécute dans le même binaire, dans le bon ordre, sans crash ?

La réponse est **oui** — et la preuve est la démonstration ci-dessous.

![Aperçu en boucle](./jour15_demo_finale.gif)  
Vidéo complète : [jour15_demo_finale.mp4](./jour15_demo_finale.mp4)

---

## Nature du Jour 15

| Aspect | Détail |
|--------|--------|
| Type | Synthèse / démo / revue rétrospective |
| Nouveau code technique | Non — réutilisation intégrale des modules Jours 4–14 |
| Nouveau comportement | Séquence de validation, couleurs structurées, test heap `0xC0FFEE`, bannière finale |
| Livrable principal | Vidéo QEMU + rapport bilan |

---

## Architecture globale du kernel final

```
                    ┌─────────────────────────────────┐
                    │         Utilisateur (clavier)    │
                    └───────────────┬─────────────────┘
                                    │ IRQ1
                    ┌───────────────▼─────────────────┐
                    │  Couche I/O : PIC + Keyboard     │  Jour 14
                    │  + Executor (poll + hlt)         │
                    └───────────────┬─────────────────┘
                                    │
                    ┌───────────────▼─────────────────┐
                    │  Couche affichage : VGA Writer   │  Jours 4-5
                    │  + Mutex + couleurs              │
                    └───────────────┬─────────────────┘
                                    │
                    ┌───────────────▼─────────────────┐
                    │  Couche mémoire : Frames + Heap  │  Jours 9-13
                    │  + Linked list allocator + NX    │  Jour 10
                    └───────────────┬─────────────────┘
                                    │
                    ┌───────────────▼─────────────────┐
                    │  Couche CPU : GDT + IDT + IST    │  Jours 7-8
                    │  + exceptions + Double Fault     │
                    └───────────────┬─────────────────┘
                                    │
                    ┌───────────────▼─────────────────┐
                    │  Boot : stage1 + stage2 (64b)  │  Jours 1-3, 6.5
                    └─────────────────────────────────┘
```

---

## Séquence d'exécution détaillée

### Phase 0 — Bannière

Le kernel efface l'écran VGA, affiche une bannière **jaune** et logue la même chose sur le port série. Double canal de sortie = diagnostic même si VGA corrompu.

### Phase 1 — Sécurité CPU `[1/4]`

```rust
gdt::init();
idt::init();
```

Initialise la **Global Descriptor Table** (segments kernel), l'**Interrupt Descriptor Table** (handlers exceptions + IRQ clavier), et la stack **IST** pour le double fault handler (Jour 8).

**Validé si :** pas de GPF/Triple Fault à ce stade.

### Phase 2 — Mémoire `[2/4]`

```rust
frame_allocator::init();
let heap_ok = heap::map_heap();
heap_allocator::init();
```

Enchaîne les trois allocateurs construits progressivement :

1. **Frame allocator** (Jour 11) — inventaire des pages physiques 2 Mo
2. **Heap mapping** (Jour 12) — insertion de 4 frames dans les tables de pages à `0x4000000`
3. **Linked list allocator** (Jour 13) — gestion alloc/free/coalescence

Puis le **test mémoire réel** :

```rust
let test_ptr = heap_allocator::alloc(128);
let mut mem_ok = heap_ok && test_ptr.is_some();
if let Some(ptr) = test_ptr {
    unsafe {
        core::ptr::write_volatile(ptr as *mut u64, 0xC0FFEE_C0FFEE);
        let val = core::ptr::read_volatile(ptr as *const u64);
        mem_ok = mem_ok && (val == 0xC0FFEE_C0FFEE);
    }
    heap_allocator::free(ptr);
}
```

Pourquoi `write_volatile` / `read_volatile` ?
- Le compilateur ne doit pas optimiser ces accès mémoire
- On teste la **vraie** RAM mappée, pas un registre optimisé away
- La valeur `0xC0FFEE_C0FFEE` est reconnaissable dans un hexdump (mot-valise « coffee »)

Affichage : **OK** (vert) ou **ECHEC** (rouge).

### Phase 3 — Écran `[3/4]`

Pas de re-init — on prouve que le **VGA Writer** (Jours 4-5) fonctionne déjà. Démo visuelle :

```
Demo couleurs : Rouge Vert Bleu Magenta
```

Chaque mot est affiché dans sa couleur via `set_color(fg, bg)`.

### Phase 4 — Clavier `[4/4]`

```rust
unsafe { pic::init(); }
unsafe { core::arch::asm!("sti"); }
```

Remappe le PIC 8259 (IRQ1 → vecteur 33), active les interruptions, puis lance l'exécuteur :

```rust
let mut executor = executor::Executor::new();
executor.run(); // boucle infinie poll + hlt
```

### Phase finale — Bannière de succès

```
======================================
  TOUS LES SOUS-SYSTEMES : OK
======================================

Tapez au clavier pour la demo finale :
```

Puis mode interactif : chaque frappe → IRQ1 → buffer circulaire → `poll()` → affichage série + VGA.

---

## Démonstration

**GIF (aperçu) :** [jour15_demo_finale.gif](./jour15_demo_finale.gif)  
**Vidéo complète :** [jour15_demo_finale.mp4](./jour15_demo_finale.mp4)

Enregistrement du 7 juillet 2026. Contenu attendu dans la vidéo :

| Timestamp approx. | Événement |
|-------------------|-----------|
| Début | Bannière jaune, phases `[1/4]` à `[4/4]` avec OK vert |
| Milieu | Ligne « Demo couleurs : Rouge Vert Bleu Magenta » |
| Avant fin | Bannière « TOUS LES SOUS-SYSTEMES : OK » |
| Fin | Frappes clavier visibles sur VGA et terminal série |

---

## Revue rétrospective — ce qu'on a construit

### Jours 1–3 : Fondations

- Rust `no_std` / `no_main`, panic handler, boot sector ASM
- Transition mode protégé puis Long Mode (Jour 6.5)
- **Cyber :** surface d'attaque minimale, pas de libstd

### Jours 4–5 : Sortie

- Accès VGA `0xb8000`, puis encapsulation safe avec `Writer`, `Mutex`, `println!`
- **Cyber :** bounds checking sur le buffer écran, pas d'écriture hors limites

### Jour 6 : Qualité

- Tests automatisés QEMU + port série
- **Cyber :** détection précoce des régressions logiques (pas seulement syntaxe)

### Jours 7–8 : Robustesse CPU

- IDT 64 bits, handlers GPF/Page Fault/Double Fault
- IST — stack dédiée pour double fault
- **Cyber :** fin des triple faults silencieux, traçabilité des crashes

### Jours 9–10 : Isolation mémoire

- Lecture tables de pages, huge pages 2 Mo
- Bit NX + `EFER.NXE` — pages données non exécutables
- **Cyber :** mitigation DEP, empêche shellcode sur heap/stack

### Jours 11–13 : Allocation dynamique

- Frame allocator, heap mapping, linked list allocator
- Tests double-free, coalescence, UAF démontré volontairement
- **Cyber :** mêmes vulnérabilités qu'un heap C — audit `unsafe` obligatoire

### Jour 14 : Interaction

- PIC remappé, IRQ1 clavier, buffer circulaire fixe, executeur cooperatif
- **Cyber :** pas de buffer dynamique = pas de heap exhaustion par flood clavier

### Jour 15 : Intégration

- Tout ci-dessus dans un flux unique de démarrage
- Test mémoire end-to-end, démo visuelle, mode interactif

---

## Bilan des zones `unsafe`

Notre kernel repose sur des blocs `unsafe` là où Rust ne peut pas garantir l'invariant matériel :

| Module | `unsafe` pour | Risque si mal implémenté |
|--------|---------------|--------------------------|
| `boot.asm` / `stage2.asm` | Init CPU, paging | Boot impossible ou GPF |
| `gdt.rs` / `idt.rs` | Tables CPU, handlers ASM | Privilege escalation, crash |
| `paging.rs` | Modification entrées PT/PD | Corruption mémoire, NX bypass |
| `heap_allocator.rs` | Gestion headers/blocs | Heap overflow → arbitrary write |
| `pic.rs` | Ports I/O PIC | IRQ fantômes, GPF |
| `keyboard.rs` | Handler IRQ | Race conditions, perte scancodes |

Rust **safe** protège le code appelant (ex. `main.rs` n'a pas besoin d'`unsafe` pour afficher), mais l'**audit du noyau** reste manuel — comme en C, avec l'avantage que les frontières safe/unsafe sont explicites.

---

## Ce qui manquerait pour un OS de production

| Domaine | État actuel | Production |
|---------|-------------|------------|
| Multitâche | Cooperatif (poll) | Préemptif (timer + scheduler) |
| Isolation | Monolithique kernel | Userspace, syscalls, MMU par process |
| Mémoire | Allocateur maison basique | jemalloc/mimalloc, guard pages, ASLR |
| Sécurité | NX, IST, IDT | KASLR, SMEP, SMAP, CFI, stack canaries |
| I/O | Clavier PS/2 | Drivers génériques, hotplug |
| Fichiers | Aucun | VFS, ext4, permissions |
| Réseau | Aucun | Stack TCP/IP, firewall |
| Mise à jour | Aucun | Signature, rollback |

Notre OS est un **laboratoire pédagogique** — pas un produit déployable. Sa valeur : comprendre chaque couche suffisamment pour **auditer** un vrai système (Linux, Windows, firmware).

---

## Lancer le kernel final

```bash
cd Day15/OS_Day15
cargo run
```

Sortie attendue (série + VGA) :

```
======================================
  JOUR 15 - SYNTHESE FINALE
======================================

[1/4] Securite CPU : OK
[2/4] Memoire : OK
[3/4] Ecran   : OK
      Demo couleurs : Rouge Vert Bleu Magenta
[4/4] Clavier : OK

======================================
  TOUS LES SOUS-SYSTEMES : OK
======================================

Tapez au clavier pour la demo finale :
```

---

## Arborescence

```
Day15/
├── jour15_demo_finale.gif              ← aperçu en boucle (README)
├── jour15_demo_finale.mp4              ← vidéo complète
├── Jour15_Synthese_Finale_Resume.md
├── Jour15_Synthese_Finale_Complet.md
└── OS_Day15/
    ├── src/main.rs                     ← point d'entrée synthèse
    ├── src/gdt.rs, idt.rs, paging.rs
    ├── src/frame_allocator.rs, heap.rs, heap_allocator.rs
    ├── src/vga.rs, vga1.rs, vga2.rs
    ├── src/pic.rs, keyboard.rs, executor.rs
    └── Cargo.toml
```

---

## Conclusion

Le Jour 15 clôt la première phase du projet : **15 jours, un kernel x86_64 en Rust qui boote, gère la mémoire, affiche en couleur et répond au clavier**. La vidéo en est la preuve tangible. La prochaine étape — le **livre** — reprendra cette progression jour par jour avec schémas, exercices cyber et ce bilan rétrospectif enrichi.
