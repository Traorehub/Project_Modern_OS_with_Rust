# Jour 17 : Bootloader LBA et contexte CPU

---

## Introduction

Le Jour 16 a donné un pouls au noyau, mais la chaîne de chargement héritée du Jour 3 plafonnait à **59 secteurs** (30 Ko). Le Jour 17 reprend le bootloader en **LBA**, puis ajoute `ThreadContext` et `switch_context` : sauvegarder une tâche, en reprendre une autre. Le préemptif du Jour 18 pourra s'appuyer dessus.

---

## Partie 1 : lecture LBA

`INT 13h AH=0x42` utilise un *Disk Address Packet* de 16 octets : nombre de secteurs, segment:offset, LBA 64 bits. Plus de cylindre, tête, secteur.

Le MBR charge d'abord `stage2` (LBA 1, 0x8000), puis le kernel par **paquets de 64 secteurs** (32 Ko) à partir de **0x10000**. Chaque paquet reste sous la frontière 64 Ko. Le segment de destination avance de `secteurs × 32` paragraphes.

`build.sh` écrit la taille réelle dans le MBR :

| Offset | Champ | Type |
|---|---|---|
| 500 | `kernel_bytes` | `u32` LE |
| 504 | `kernel_sectors` | `u16` LE |

`stage2` lit `kernel_bytes` à `0x7DF4` et copie **exactement** ce nombre d'octets (arrondi au dword) de `0x10000` vers `0x200000`. Fini les 128 Ko fictifs.

Trois essais par lecture, reset disque entre deux. Messages BIOS `A20`, `LBA`, `DISK` en cas d'échec.

A20 : Fast Gate (`0x92`), puis `INT 15h AX=0x2401`, puis 8042. Un test `0000:0500` contre `FFFF:0510` vérifie que la ligne est vraiment ouverte.

Nouveau plafond : **768 secteurs (384 Ko)**. Au-delà, le tampon `0x10000` toucherait les tables de pages à `0x70000`.

---

## Partie 2 : `switch_context`

```
r15 r14 r13 r12 rbx rbp   (callee-saved SysV)
rsp rip rflags
```

`switch_context(old, new)` écrit l'état courant dans `old`, charge `new`, saute à `new.rip`. Une tâche neuve a `RIP = entry`, `RSP` aligné SysV (`top - 8`), `RFLAGS = 0x202`.

Démo : `main -> A -> B -> A -> main`. Séquence attendue : **ABA**, 3 commutations. Deux piles de 4 Ko.

Ce n'est **pas** encore du préemptif : personne n'appelle `switch` depuis IRQ0. Le Jour 18 le fera, et devra aussi sauver l'état caller-saved plus FXSAVE (SSE).

---

## Les 4 fichiers nouveaux ou réécrits

| Fichier | Rôle |
|---|---|
| `src/boot.asm` | **Réécrit** : LBA, A20, erreurs, taille dans le MBR |
| `src/stage2.asm` | **Modifié** : copie à la taille réelle depuis `0x10000` |
| `src/switch_context.asm` | **Nouveau** : commutation 64 bits |
| `src/context.rs` | **Nouveau** : `ThreadContext`, `prepare`, démo ABA |

---

## Validation

### Build LBA

![Build : 22536 octets = 45 secteurs, plafond 768](./sortie_du_build_sh.png)

```
Image kernel : 22536 octets = 45 secteurs (max 768)
OK : MBR patche, 45 secteurs LBA a charger (plafond 768)
```

Au Jour 16 le même noyau tenait de justesse sous 59 secteurs. Ici 45 secteurs sur 768 : la marge n'est plus le sujet.

### Sequence ABA

![cargo run : [6/6] OK, 3 commutations ABA](./sortit_avec_6_6ABA.png)

```
[6/6] Contexte CPU : switch_context A <-> B...
  main -> A
  [A] premiere execution, pile A
  [B] execution sur une autre pile
  [A] retour apres B, registres et pile restaures
  retour dans main
  OK - 3 commutations, sequence ABA
```

---

## Lancer

Compiler sur Kali :

```bash
cd /home/kali/Day17/OS_Day17
sed -i 's/\r$//' *.sh
chmod +x build.sh run_tests.sh run_demo.sh patch_boot_size.sh
./build.sh
cargo run
```

Ramener l'image bootable vers Windows (à mettre dans le dépôt pour les visiteurs) :

```powershell
scp root@192.168.126.129:/home/kali/Day17/os_day17.img C:\Users\MOH\Documents\Project_OS_Rust\Day17\
```

Visiteur, sans Rust ni NASM :

```bash
qemu-system-x86_64 -drive format=raw,file=os_day17.img -serial stdio
```

C'est un **disque brut MBR**, pas un ISO. Le bootloader parle LBA à un disque, pas à un CD El Torito.

**Critère de réussite :** `max 768` au build, `[6/6] ... sequence ABA` à l'exécution.

---

## Ce que ça débloque

- **Jour 18** : `schedule()` depuis le handler IRQ0, passage au **préemptif**
