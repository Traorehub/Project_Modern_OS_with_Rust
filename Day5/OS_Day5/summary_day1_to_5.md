# Synthèse du projet – Jours 1 à 5

## 📅 Jour 1 – Le bootloader minimaliste
- **Objectif** : charger un petit noyau 32 bits depuis le disque en mode réel.
- **Techniques clés** :
  - Assembly `boot.asm` qui initialise les registres, active le mode protégé et effectue `jmp 0x08:protected_mode`.
  - Placement du code à l’adresse physique `0x8000` via le **linker script**.
- **Paradigme** : *bare‑metal* – aucun OS, aucune runtime.
- **Cyber‑focus** : le bootloader n’a aucun accès aux privilèges du CPU ; il ne fait que préparer l’environnement, limitant ainsi la surface d’attaque.

## 📅 Jour 2 – Eliminer `std` et configurer la cible
- **Objectif** : compiler un kernel `#![no_std]` et `#![no_main]`.
- **Techniques clés** :
  - Utilisation de `rustc` avec la cible `i686-unknown-none.json`.
  - `panic = "abort"` pour éviter le code de gestion de panics lourd.
  - `core::arch::asm!` pour les accès bas‑niveau (ex : écriture série).
- **Paradigme** : *Zero‑cost abstractions* – on garde la sémantique Rust tout en éliminant le runtime.
- **Cyber‑focus** : réduction de la **surface d’exposition** (pas de libstd, pas d’allocation dynamique).

## 📅 Jour 3 – Gestion des panics et sortie série
- **Objectif** : afficher des messages de panic sur le port série et sur l’écran.
- **Techniques clés** :
  ```rust
  unsafe fn write_serial(byte: u8) {
      core::arch::asm!("out dx, al", in("dx") 0x3F8, in("al") byte);
  }
  ```
  - Implémentation d’un `panic_handler` qui change la couleur du texte et utilise `println!` (à venir).
- **Paradigme** : *debug‑first* – les messages de panic sont essentiels pour le diagnostic sans debugguer un VM.
- **Cyber‑focus** : journalisation **hors‑processus** (port série) garantit l’intégrité du log même en cas de plantage du kernel.

## 📅 Jour 4 – Accès direct au buffer VGA (`0xb8000`)
- **Objectif** : écrire du texte couleur sur l’écran en écrivant directement dans la mémoire vidéo.
- **Techniques clés** :
  - Pointeur brut `const VGA_BUFFER: *mut u8 = 0xb8000 as *mut u8;`
  - Fonctions `unsafe fn write_char(row, col, byte, color)` avec `core::ptr::write_volatile`.
  - **Bounds‑checking** : on ignore les coordonnées hors limites (`if row >= VGA_HEIGHT { return; }`).
- **Paradigme** : *unsafe‑minimal* – l’`unsafe` est limité à un petit module.
- **Cyber‑focus** : **Isolation du code dangereux** (single `unsafe` block) prévient les corruptions mémoire et les débordements.

## 📅 Jour 5 – Safe Wrapper, Spinlock & `println!`
- **Objectif** : fournir une API sûre (`Writer`) et une macro `println!` similaire à Rust std.
- **Techniques clés** :
  - **Split du module** : `vga1.rs` = spinlock (`Mutex<T>` basé sur `AtomicBool::swap`); `vga2.rs` = logique d’affichage (`Writer`).
  - **Mutex global** : `pub static WRITER: Mutex<Writer> = Mutex::new(Writer::new());`
  - Implémentation de `core::fmt::Write` pour permettre `write!` & `println!`.
  - **Scroll** automatisé quand on dépasse la hauteur du texte.
  - **Linker fix** : wildcards `*(.text.*)` etc. pour que `_start` reste exactement à `0x8000`.
- **Paradigme** : *safe abstraction* – le noyau utilise uniquement du code sûr, les `unsafe` sont confinés au bas‑niveau.
- **Cyber‑focus** :
  - **Principe du moindre privilège** : le reste du kernel n’interagit plus directement avec la mémoire matérielle.
  - **Protection contre les data races** : spinlock garantit qu’une seule partie du code écrit dans le buffer à la fois.
  - **Vérification des limites** : tous les accès sont bornés, éliminant les débordements.
  - **Auditabilité** : l’API `println!` facilite la journalisation et le suivi des événements, indispensable en sécurité.

---

### 📚 Compétences consolidées
| Domaine | Compétence | Exemple concret |
|---|---|---|
| **Assembly & boot** | Écriture d’un bootloader minimal, passage en mode protégé | `boot.asm` → `jmp 0x08:protected_mode` |
| **Rust bare‑metal** | `#![no_std]`, `#![no_main]`, `panic=abort` | `Cargo.toml` & `i686-unknown-none.json` |
| **Gestion du hardware** | Port série, buffer VGA, manipulations de registres | `write_serial`, `VGA_BUFFER` |
| **Sécurité système** | Isolation d’`unsafe`, spinlock maison, vérifications de limites | `vga1.rs` / `write_cell` |
| **Débogage** | Panic handler, journalisation série, macros `println!` | `panic_handler` + `println!` |

---

### 🔐 Point cyber‑sécurité
1. **Isolation du code dangereux** – tout le `unsafe` est confined dans un seul fichier (`vga1.rs`/`vga2.rs`).
2. **Contrôle d’accès concurrent** – spinlock empêche les data‑races, garantissant l’intégrité de l’affichage.
3. **Contrôle des limites** – chaque écriture vérifie `row < VGA_HEIGHT && col < VGA_WIDTH`.
4. **Journalisation fiable** – sortie série même en cas de panic, permettant une analyse post‑mortem.
5. **Surface d’attaque minimale** – aucune dépendance réseau, tout le code est produit localement.

---

**Conclusion** : en cinq jours nous avons construit un mini‑OS capable de démarrer, de gérer le texte couleur sur l’écran, de logger sur le port série, et surtout d’appliquer les bons principes de *sécurité système* : moindre privilège, protection contre les data‑races et vérifications de limites. Le prochain pas logique sera d’ajouter la gestion des interruptions (IDT) et un allocateur de mémoire, toujours en conservant ces principes de sécurité.
