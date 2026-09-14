# Jour 18 : Scheduler préemptif (Round-Robin)

---

## Introduction

Au Jour 17, A et B se passaient la main **volontairement**. Un `loop {}` sans `switch` gardait le CPU pour toujours. Le Jour 18 appelle `schedule()` depuis le handler **IRQ0**. Le timer coupe la tâche, même si elle refuse.

---

## L'idée

Trois tâches : MAIN (la boucle d'attente, puis l'exécuteur), A et B. A et B font :

```
loop { compteur += 1; }   // jamais de switch, jamais de hlt
```

Toutes les **10 ticks** (100 ms à 100 Hz), IRQ0 fait EOI puis `schedule()` : on sauve le contexte courant, on prend le suivant (0 -> 1 -> 2 -> 0).

Si après 2 secondes `A > 0` **et** `B > 0`, les deux ont eu du CPU **sans avoir demandé**. C'est la définition du préemptif.

---

## Pourquoi l'EOI avant le switch

Si on commute **avant** `send_eoi(0)`, le PIC croit qu'IRQ0 est encore en cours. Plus aucun tick, plus de préemption, gel. Ordre : tick, EOI, `schedule()`.

Toujours **aucun verrou VGA** dans le handler.

---

## FXSAVE

IRQ0 peut tomber au milieu d'une instruction SSE. On sauve 512 octets (`fxsave64` / `fxrstor64`) en plus des registres callee-saved. Image FX initiale : `FCW = 0x037F`, `MXCSR = 0x1F80`.

---

## Ce qui change

| Fichier | Rôle |
|---|---|
| `src/scheduler.rs` | **Nouveau** : file Round-Robin, A/B, quantum 10 ticks |
| `src/switch_context.asm` | **Modifié** : `fxsave64` / `fxrstor64` |
| `src/context.rs` | **Modifié** : buffer FX 512 o, aligné 16 |
| `src/idt.rs` | **Modifié** : `scheduler::on_timer_tick()` après EOI |
| `src/main.rs` | Phase `[6/6]` : attendre 2 s, vérifier A et B |
| `src/executor.rs` | Barre d'état : compteurs A et B |

---

## Lancer

```bash
cd /home/kali/Day18/OS_Day18
sed -i 's/\r$//' *.sh
chmod +x build.sh run_tests.sh run_demo.sh patch_boot_size.sh
./build.sh
cargo run
```

Image visiteurs, après le build :

```powershell
scp root@192.168.126.129:/home/kali/Day18/os_day18.img C:\Users\MOH\Documents\Project_OS_Rust\Day18\
```

```bash
qemu-system-x86_64 -drive format=raw,file=os_day18.img -serial stdio
```

**Critère :** `[6/6] Preempt : OK` avec `A` et `B` tous les deux non nuls, plus `switches >= 2`.

---

## Validation

Mesure réelle sur Kali (`cargo run`). Quantum 10 ticks = 100 ms. Environ **10 switches/s**.

### Instant 1 : juste après les 2 s de test

![QEMU : A et B deja tous les deux au-dessus de 3 millions, 21 switches](./rendu_A_B_instant1.png)

```
[6/6] Scheduler : A et B bouclent sans yield...
  taches A et B armees (boucle infinie, aucun switch)
  attente 2 s : IRQ0 doit les couper de force...
  OK - A=3114722 B=3109425 switches=21 (aucun yield volontaire)

[uptime] 2 s - 211 ticks - A=3114722 B=3109425 sw=21 - 0 touches
```

A et B sont au même ordre de grandeur (3,11 M / 3,10 M). Aucun des deux n'a monopolisé le CPU.

### Instant 2 : 30 s plus tard, toujours sans yield

![QEMU : A et B ont continue de monter, 300 switches](./rendu_A_B_instant2.png)

```
[uptime] 30 s - 3001 ticks - A=39259465 B=38590626 sw=300 - 0 touches
```

A passe de 3,1 M à 39,3 M, B de 3,1 M à 38,6 M. `sw` suit le quantum (21 à 2 s, 300 à 30 s). Les deux boucles `loop { compteur += 1 }` n'appellent jamais `switch` : seul IRQ0 les coupe.

---

## Suite

Jours 19-20 : syscalls et Ring 3. L'ordonnanceur ne gère encore que des tâches noyau.
