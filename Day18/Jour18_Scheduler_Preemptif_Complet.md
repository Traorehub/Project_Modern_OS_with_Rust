# Jour 18 : du coopératif au préemptif

---

## Introduction

Le Jour 17 a prouvé qu'on savait sauver une pile et en reprendre une autre. Mais A appelait `switch` vers B. C'est du **coopératif** : la tâche rend la main quand elle veut. Un processus hostile, ou simplement un `loop {}` oublié, gèle la machine.

Le Jour 16 a donné le timer. Le Jour 18 s'en sert : **IRQ0 décide** qui tourne. A et B n'ont plus le choix.

---

## Round-Robin à trois

| Slot | Tâche | Comportement |
|---|---|---|
| 0 | MAIN | attend 2 s (`hlt`), puis exécuteur clavier / uptime |
| 1 | A | `loop { COUNTER_A += 1 }` |
| 2 | B | `loop { COUNTER_B += 1 }` |

Quantum : **10 ticks = 100 ms**. À 100 Hz, on tourne A, B, MAIN, A, ...

Le premier `switch_context` depuis l'IRQ sauve MAIN (qui était dans le handler, pile noyau) dans le slot 0, puis saute dans A. Quand MAIN reprend, le handler termine, `iret` rend la main à la boucle d'attente. Pas besoin de préparer le contexte de MAIN à l'avance.

---

## Le handler IRQ0

```
timer::on_tick();
pic::send_eoi(0);
scheduler::on_timer_tick();   // peut appeler switch_context
```

L'EOI est **avant** le switch. Sinon le PIC garde IRQ0 occupée : plus de ticks, plus de rotation, symptôme « le préemptif a tué le timer ».

`on_timer_tick` ne prend **aucun** verrou. Un `println!` VGA ici recréerait le deadlock du Jour 16.

---

## FXSAVE

`x86_64-unknown-none` émet du SSE. Une préemption au milieu d'un `movaps` sans sauver XMM corrompt l'autre tâche (ou lève `#UD` / #XM). `ThreadContext` commence par 512 octets alignés 16. `switch_context` fait `fxsave64` / `fxrstor64`.

Une zone FX toute à zéro n'est pas toujours restaurable. À la création d'une tâche on pose `FCW = 0x037F` et `MXCSR = 0x1F80` (état x87/SSE au reset).

---

## La preuve

A et B n'appellent jamais `switch`. Si après 2 secondes les deux compteurs ont avancé, seul IRQ0 a pu leur donner le CPU. Mesure réelle sur Kali (`cargo run`).

### Instant 1 : fin du test `[6/6]` (2 s)

![Compteurs A et B apres 2 s de preemption forcee](./rendu_A_B_instant1.png)

```
[6/6] Scheduler : A et B bouclent sans yield...
  taches A et B armees (boucle infinie, aucun switch)
  attente 2 s : IRQ0 doit les couper de force...
  OK - A=3114722 B=3109425 switches=21 (aucun yield volontaire)
```

21 switches en 2 s : le quantum de 10 ticks (100 ms à 100 Hz) donne bien ~10 rotations par seconde. A et B sont collés (3 114 722 vs 3 109 425). Le Round-Robin n'a pas favorisé l'un des deux.

### Instant 2 : l'exécuteur tourne, A et B continuent

![Uptime 30 s : A et B ont encore monte, sw=300](./rendu_A_B_instant2.png)

```
[uptime]  2 s -  211 ticks - A= 3114722 B= 3109425 sw= 21 - 0 touches
[uptime] 30 s - 3001 ticks - A=39259465 B=38590626 sw=300 - 0 touches
```

| Instant | A | B | switches | ticks |
|---|---|---|---|---|
| 2 s | 3 114 722 | 3 109 425 | 21 | 211 |
| 30 s | 39 259 465 | 38 590 626 | 300 | 3001 |

A ×12,6, B ×12,4, `sw` ×14,3. Les deux restent dans le même ordre de grandeur. Personne n'a gelé, personne n'a tout pris. L'exécuteur affiche l'uptime et accepte le clavier pendant que IRQ0 continue de couper A et B.

---

## Ce que ça ne fait pas encore

- Pas de priorités, pas de sleep, pas de join.
- Trois tâches noyau seulement, pas de processus utilisateur.
- Pas de protection : A pourrait écraser la pile de B.
- Ring 3 et syscalls : Jours 19-20.

---

## Angle Cyber

| Mécanisme | Enjeu |
|---|---|
| Préemption | Un `while(true)` en userspace ne gèle plus la machine (une fois Ring 3 en place) |
| Quantum trop court | Le CPU passe son temps dans les handlers : DoS |
| Switch depuis l'IRQ sans EOI | Toute une ligne d'interruptions meurt |
| FXSAVE oublié | Fuite ou corruption d'état SSE entre tâches |
| Handler qui prend un verrou | Deadlock, déjà vu au Jour 16 |

---

## Lancer

```bash
cd /home/kali/Day18/OS_Day18
sed -i 's/\r$//' *.sh
chmod +x build.sh run_tests.sh run_demo.sh patch_boot_size.sh
./build.sh
cargo run
```

```powershell
scp root@192.168.126.129:/home/kali/Day18/os_day18.img C:\Users\MOH\Documents\Project_OS_Rust\Day18\
```

```bash
qemu-system-x86_64 -drive format=raw,file=os_day18.img -serial stdio
```

---

## Arborescence

```
Day18/
├── Jour18_Scheduler_Preemptif_Resume.md
├── Jour18_Scheduler_Preemptif_Complet.md
├── rendu_A_B_instant1.png       <- [6/6] OK, A et B a 2 s
├── rendu_A_B_instant2.png       <- uptime 30 s, A et B ont monte
├── os_day18.img                 <- apres ./build.sh + scp
└── OS_Day18/
    └── src/
        ├── scheduler.rs         <- NOUVEAU
        ├── switch_context.asm   <- FXSAVE
        ├── context.rs           <- buffer FX
        ├── idt.rs               <- schedule depuis IRQ0
        └── main.rs              <- demo 2 s
```
