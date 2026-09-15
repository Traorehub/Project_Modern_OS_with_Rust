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

### Build

![Build : 22776 octets = 45 secteurs, plafond 768](./sortie_du_build_sh.png)

```
Image kernel : 22776 octets = 45 secteurs (max 768)
OK : MBR patche, 45 secteurs LBA a charger (plafond 768).
     Image visiteurs : ../os_day18.img
```

22 536 octets au Jour 17, 22 776 aujourd'hui : le scheduler et FXSAVE tiennent. Toujours 45 secteurs sur 768.

### Instant 1 : 10 s, A et B deja colles

![Uptime 10 s : A=14517171 B=14179511 sw=99](./imagea10secondeaveclesvaleurdeAetBquetuconnais.png)

```
[6/6] Scheduler : A et B bouclent sans yield...
  OK - A=3114722 B=3109425 switches=21 (aucun yield volontaire)

[uptime] 10 s - 1000 ticks - A=14517171 B=14179511 sw=99 - 0 touches
```

### Instant 2 : 22 s, A et B ont continue de monter

![Uptime 22 s : A=29160002 B=28801581 sw=219](./imagea22secondeaveclesvaleurdeAetBquetuconnais.png)

```
[uptime] 22 s - 2200 ticks - A=29160002 B=28801581 sw=219 - 0 touches
```

Quantum 10 ticks = 100 ms, environ **10 switches/s** (99 à 10 s, 219 à 22 s). A et B restent dans le même ordre de grandeur. Aucun yield volontaire.

---

## Suite

Jours 19-20 : syscalls et Ring 3. L'ordonnanceur ne gère encore que des tâches noyau.
