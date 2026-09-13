# Rapport de Validation - Jour 6.5 : Switch 32 bits → 64 bits (Long Mode x86_64)

---

## Transition depuis le Jour 6

Au Jour 6, tous les tests passaient en **mode protégé 32 bits** (`i686`). Le kernel, le VGA, le port série et l'infrastructure de test fonctionnaient correctement.

Le Jour 7 introduit l'IDT (Interrupt Descriptor Table) et les exceptions CPU - ces mécanismes sont fondamentalement différents en 32 et 64 bits, et les ressources modernes (CVEs, exploits, mitigations) ciblent exclusivement le 64 bits. Ce jour intermédiaire effectue donc la migration complète vers le **Long Mode x86_64** avant d'aborder l'IDT.

---

## Pourquoi on a commencé en 32 bits

Commencer en 32 bits n'est pas une erreur - c'est la progression naturelle et pédagogiquement solide :

```
32 bits (Jours 2-6) :
✅ Boot sector simple - pas de paging obligatoire
✅ no_std, panic handler, VGA, Mutex, println! → fondamentaux acquis
✅ Tests automatisés QEMU → infrastructure de qualité
→ Tous les concepts de base sans complexité inutile

64 bits (Jour 6.5+) :
→ Long Mode sur des bases solides
→ Paging obligatoire - compris et implémenté manuellement
→ IDT 64 bits, exceptions CPU modernes
→ Mitigations de sécurité réelles (SMEP, SMAP, KASLR)
```

C'est exactement la progression de Linux (32 bits en 1991, 64 bits en 2001), xv6 (MIT) et SerenityOS.

---

## Différence fondamentale 32 bits vs 64 bits

### Architecture

```
32 bits (Protected Mode) :
- Registres : EAX, EBX, ECX... (32 bits)
- Espace mémoire max : 4 Go
- Paging : optionnel
- Segments : obligatoires pour la protection mémoire

64 bits (Long Mode) :
- Registres : RAX, RBX, RCX... (64 bits)
- Espace mémoire : 128 To en pratique
- Paging : OBLIGATOIRE pour entrer en Long Mode
- Segments : quasi inutilisés (flat memory model)
```

### Impact cyber

```
32 bits :
- ASLR faible (peu d'entropie)
- Heap spray possible
- Exploits plus prévisibles

64 bits :
- ASLR fort (47 bits d'entropie)
- Heap spray quasi impossible
- Mitigations : NX, SMEP, SMAP, CET, KASLR
- Exploits modernes : Spectre, Meltdown, Rowhammer
```

---

## Les étapes obligatoires pour entrer en Long Mode

```
1. Mode Réel 16 bits (démarrage BIOS)
        ↓
2. Activer la ligne A20 (accès mémoire > 1Mo)
        ↓
3. Charger GDT 32 bits + passer en Mode Protégé
        ↓
4. Activer PAE (Physical Address Extension)
        ↓
5. Créer les tables de pages (PML4 → PDPT → PD)
        ↓
6. Activer le paging (CR0.PG)
        ↓
7. Activer Long Mode (EFER.LME)
        ↓
8. Charger GDT 64 bits + saut long
        ↓
9. Kernel Rust s'exécute en 64 bits ✅
```

### Le paging - pourquoi obligatoire en 64 bits

```
Sans paging (32 bits) :
programme → adresse physique directe

Avec paging (64 bits) :
programme → adresse virtuelle → PML4 → PDPT → PD → physique
```

On utilise des **huge pages 2Mo** (PD → physique directement) pour simplifier - on évite le niveau PT.

### Tables de pages utilisées

```
0x1000 → PML4  : PML4[0] → PDPT à 0x2000
0x2000 → PDPT  : PDPT[0] → PD à 0x3000
0x3000 → PD    : PD[0]   → 0x000000 (2Mo, identité map)
                 PD[1]   → 0x200000 (2Mo, adresse du kernel)
```

---

## Architecture en deux secteurs

Le boot sector (512 octets) ne peut pas contenir tout le code de transition. On sépare donc en deux fichiers :

```
os.img :
├── Secteur 1 (512o) → boot.bin   : code 16 bits, charge stage2+kernel
├── Secteur 2 (512o) → stage2.bin : code 32 bits → 64 bits, active Long Mode
└── Secteur 3+       → kernel.bin : kernel Rust 64 bits
```

---

## Ce qui change par rapport au Jour 6

```
Supprimé :
❌ i686-unknown-none.json    (target custom 32 bits)
❌ boot.asm monolithique     (trop grand pour 512 octets)

Ajouté :
✅ src/stage2.asm            (code 32 bits → 64 bits séparé)

Modifié :
🔄 src/boot.asm              (16 bits uniquement, charge stage2+kernel)
🔄 linker.ld                 (ELF64, adresse 0x200000)
🔄 .cargo/config.toml        (target x86_64-unknown-none)
🔄 Cargo.toml                (test=false pour éviter conflit core)
🔄 run_tests.sh              (inclut stage2.bin, seek=2 pour kernel)

Inchangé :
✅ src/vga1.rs, vga2.rs, vga.rs
✅ src/serial.rs
✅ src/test_runner.rs
✅ src/main.rs
✅ tests/src/ (tout)
```

---

## Points critiques - Ce qu'il faut absolument retenir

* **cargo test vs build manuel** : `cargo test` dans un workspace `no_std` cause des conflits de `core` en raison de flags de compilation divergents. La solution est de compiler avec `cargo build` et de lancer le script runner manuellement.
* **test = false dans Cargo.toml** : Requis pour empêcher Cargo de compiler des tests intégrés autonomes pour le binaire du kernel principal, ce qui entrerait en conflit avec la suite `tests/`.
* **kernel à 0x200000** : Impossible de charger directement à cette adresse en mode réel (limite à 1 Mo). Le boot sector charge à `0x8000`, puis `stage2` effectue la copie vers `0x200000` en mode protégé.
* **objcopy avec --only-section** : Permet d'éviter de générer un binaire plat gigantesque contenant 2 Mo de zéros d'initialisation en extrayant sélectivement uniquement les sections de code nécessaires.

---

## Commandes complètes - Migration & Run

### Étape 1 - Compiler les binaires ASM
```bash
nasm -f bin src/boot.asm -o boot.bin
nasm -f bin src/stage2.asm -o stage2.bin
```

### Étape 2 - Compiler le kernel
```bash
cargo build -Z build-std=core --target x86_64-unknown-none
```

### Étape 3 - Vérifier l'adresse de `_start`
```bash
nm target/x86_64-unknown-none/debug/kernel | grep _start
# Doit afficher : 0000000000200000 T _start
```

### Étape 4 - Créer l'image et lancer QEMU
```bash
objcopy -O binary \
  --only-section=.text \
  --only-section=.rodata \
  --only-section=.data \
  --only-section=.bss \
  target/x86_64-unknown-none/debug/kernel kernel.bin

dd if=/dev/zero   of=os.img bs=512 count=8192
dd if=boot.bin    of=os.img conv=notrunc
dd if=stage2.bin  of=os.img bs=512 seek=1 conv=notrunc
dd if=kernel.bin  of=os.img bs=512 seek=2 conv=notrunc

qemu-system-x86_64 -drive format=raw,file=os.img -serial stdio -display none
```

### Étape 5 - Lancer les tests
```bash
cargo build -Z build-std=core --target x86_64-unknown-none -p tests
./run_tests.sh target/x86_64-unknown-none/debug/kernel_tests
```

---

## Résultats obtenus

Le passage au Long Mode 64 bits s'est effectué avec succès. On observe le bon chargement et l'adresse de `_start` à `0x200000` :

![Résultats du passage au 64 bits et validation de l'adresse de _start](./switch_64bit_results.png)

---

## Résumé - Angle Cyber

| Concept | 32 bits (Jours 2-6) | 64 bits (Jour 6.5+) |
|---|---|---|
| Espace mémoire | 4 Go max | 128 To en pratique |
| ASLR | Faible entropie | Forte entropie (47 bits) |
| Paging | Optionnel | Obligatoire |
| Isolation mémoire | Segments | Pages + SMEP/SMAP |
| Heap spray | Possible | Quasi impossible |
| Exploits modernes | Rares | Spectre, Meltdown, Rowhammer |
| IDT | Structure 32 bits | Structure 64 bits (Jour 7) |

> Le paging obligatoire en 64 bits est la **base de toutes les mitigations modernes**. Comprendre comment on l'active manuellement dans le boot sector c'est comprendre exactement ce qu'un **bootkit** doit contourner pour compromettre un système avant le démarrage de l'OS - c'est précisément ce que font des malwares comme BlackLotus ou FinSpy.
