# Jour 14 - Async/Await : Driver Clavier

---

## Introduction

Au Jour 13 on avait un heap allocateur. Au Jour 14 on passe au **multitache cooperatif** : un executeur poll en boucle un driver clavier base sur les interruptions IRQ1. Le PIC 8259 est remappe pour eviter le conflit IRQ/exception, et un buffer circulaire relie le handler d'interruption a l'affichage.

---

## Architecture

```
[Touche clavier]
      |
   IRQ1 (PIC remappe -> vecteur 33)
      |
  keyboard_handler()
      |
  SCANCODE_QUEUE.push()    <- buffer circulaire 64 scancodes
      |
  Executor::run() loop
      |
  KeyboardFuture::poll()
      |
  scancode_to_ascii()
      |
  serial + VGA
```

---

## Cle du remappage PIC

```rust
pub const PIC1_OFFSET: u8 = 32; // IRQ0 -> vecteur 32
pub const PIC2_OFFSET: u8 = 40; // IRQ8 -> vecteur 40
// IRQ1 clavier -> vecteur 33

// Masque : seul IRQ1 actif
outb(PIC1_DATA, 0xFD); // 11111101 -> IRQ1 seul
outb(PIC2_DATA, 0xFF); // tout masque sur PIC2
```

Sans ce remappage : IRQ1 -> vecteur 9 -> General Protection Fault -> crash immediat.

---

## Buffer circulaire - sans allocation

```rust
pub struct ScancodeQueue {
    buffer: [u8; 64],
    head: usize,
    tail: usize,
    count: usize,
}

// push() depuis le handler IRQ
// pop() depuis le poll de l'executeur
// Queue pleine -> scancode perdu (comportement deterministe)
```

---

## Boucle d'execution

```rust
pub fn run(&mut self) -> ! {
    loop {
        if let Some(ch) = self.keyboard_task.poll() {
            // afficher ch sur serie et VGA
        }
        unsafe { core::arch::asm!("hlt"); } // attendre la prochaine IRQ
    }
}
```

`hlt` = CPU en veille jusqu'a la prochaine interruption. Sans lui : 100% CPU inutile.

---

## Arborescence

![Arborescence des fichiers du projet Jour 14](./jour14_capture1_arborescence.png)

```
OS_Day14/src/
├── main.rs          <- init GDT/IDT/PIC, sti, lance Executor
├── keyboard.rs      <- NOUVEAU : ScancodeQueue + KeyboardFuture
├── executor.rs      <- NOUVEAU : boucle poll + hlt
├── pic.rs           <- NOUVEAU : remappage PIC 8259
├── idt.rs           <- handler IRQ1
└── ...
```

---

## Resultat

### Ecran d'accueil avant frappes

![Ecran d'accueil QEMU - kernel demarre, attente de frappes](./jour14_capture2_ecran_accueil_qemu.png)

Le kernel affiche "Pret. Tapez au clavier !" en VGA. PIC configure, `sti` actif.

### Erreur GPF avant fix du PIC

![Terminal - General Protection Fault avant remappage PIC](./jour14_capture3_erreur_gpf_avant_fix.png)

Sans remappage, IRQ1 -> vecteur 9 -> pas de handler -> GPF -> crash. Fix : appeler `pic::init()` avant `sti` et enregistrer le handler IDT au vecteur 33.

### Clavier fonctionnel

![QEMU + terminal - touches affichees serie et VGA](./jour14_capture4_clavier_fonctionnel.png)

Apres fix : chaque touche -> IRQ1 -> `push(scancode)` -> `poll()` -> `scancode_to_ascii()` -> affichage serie ("Touche : a") et VGA.

---

## Angle Cyber

| Mecanisme | Risque / Protection |
|---|---|
| PIC remappe | Evite confusion IRQ/exception - defend l'injection d'IRQ |
| Buffer taille fixe | Pas de debordement dynamique possible |
| Bit 7 ignore (key release) | Filtre les scancodes parasites |
| `hlt` dans boucle | 0% CPU inutile, side-channel timing reduit |
| Queue pleine = perte deterministe | Pas de crash, comportement previsible |

> Un driver clavier sans remappage PIC est une surface d'attaque : une IRQ injectee sur le vecteur 9 (sans handler) provoque un GPF exploitable. Notre IDT avec le handler GPF defini et le PIC correctement configure ferme ce vecteur d'attaque. Le buffer circulaire a taille fixe elimine les risques de heap exhaustion par flood de frappes.
