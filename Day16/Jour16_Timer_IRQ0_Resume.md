# Jour 16 : Timer PIT 8254 (IRQ0)

---

## Introduction

Au Jour 14, le noyau ne se réveillait qu'à une frappe clavier : sans interaction, le `hlt` de la boucle principale le figeait indéfiniment. Au Jour 16 on programme le **PIT 8254** pour déclencher **IRQ0** 100 fois par seconde. Le noyau acquiert une **notion du temps**, prérequis obligatoire du multitâche préemptif (Jour 18).

---

## Le calcul du diviseur

```
Frequence de base du PIT : 1 193 182 Hz  (gravee, non modifiable)

diviseur = 1193182 / 100 = 11931  ->  ~100,006 Hz
1 tick = 10 ms
```

Le PIT décrémente un compteur à chaque oscillation ; à zéro il lève IRQ0 et recharge le diviseur.

---

## Programmation du PIT

```rust
const PIT_COMMAND: u16       = 0x43;
const PIT_CHANNEL0_DATA: u16 = 0x40;
const PIT_MODE3_SQUARE_WAVE: u8 = 0x36;

pub unsafe fn init() {
    let divisor = (1_193_182 / 100) as u16;
    outb(PIT_COMMAND, PIT_MODE3_SQUARE_WAVE);
    outb(PIT_CHANNEL0_DATA, (divisor & 0xFF) as u8); // octet bas
    outb(PIT_CHANNEL0_DATA, (divisor >> 8) as u8);   // octet haut
}
```

L'octet `0x36` se décompose en : canal 0, accès lobyte/hibyte, mode 3 (onde carrée), compteur binaire.

---

## Compteur atomique

```rust
pub static TICKS: AtomicU64 = AtomicU64::new(0);

pub fn on_tick() {
    TICKS.fetch_add(1, Ordering::Relaxed);
}
```

Écrit depuis une interruption, lu depuis la boucle principale : sans `Atomic`, le compilateur pourrait garder la valeur en registre et ne jamais voir les mises à jour.

---

## Les 4 modifications

| Fichier | Changement |
|---|---|
| `src/timer.rs` | **Nouveau** : PIT, compteur `TICKS`, helpers uptime |
| `src/pic.rs` | Masque `0xFD` → **`0xFC`** (IRQ0 + IRQ1) |
| `src/idt.rs` | Ajout `idt[32].set_handler_fn(timer_interrupt_handler)` |
| `src/main.rs` | `mod timer;`, `timer::init()`, vérification IRQ0 |

---

## Le handler, volontairement minimal

```rust
extern "x86-interrupt" fn timer_interrupt_handler(_frame: InterruptStackFrame) {
    timer::on_tick();
    unsafe { pic::send_eoi(0); } // IRQ0
}
```

---

## Les 3 pièges

### EOI oublié

Sans `send_eoi(0)`, le PIC n'envoie plus jamais IRQ0. Et comme **IRQ0 est prioritaire sur IRQ1**, le clavier meurt aussi. Symptôme trompeur : « j'ai ajouté le timer et le clavier ne marche plus ».

### Deadlock du Mutex VGA

```
Boucle principale : WRITER.lock()  <- detient le spinlock
        |  IRQ0 arrive
Handler timer     : WRITER.lock()  <- spin en attendant...
        |
Le detenteur est interrompu et ne reprendra qu'apres le handler
        |
FIGE DEFINITIVEMENT
```

**Règle :** un handler ne prend jamais un verrou partagé avec le code normal. Il incrémente le compteur, la boucle principale affiche. Si besoin ailleurs : `x86_64::instructions::interrupts::without_interrupts(|| ...)`.

### `RIP : 0x3`, appel indirect vers l'adresse 0

Symptôme rencontré en vrai, pile au premier `{}` sur un entier :

```
  PIT programme : diviseur
[EXCEPTION] Opcode invalide !
  RIP : 0x3
```

`0x3` n'est pas une adresse du noyau (chargé à `0x200000`) : le CPU a **sauté à 0**, exécuté la table des vecteurs BIOS et levé `#UD`. Donc un pointeur de fonction lu à zéro.

**Deux causes cumulées :**

| Cause | Détail | Correctif |
|---|---|---|
| Sections filtrées | `objcopy --only-section` excluait `.got` et `.data.rel.ro`, où `lld` place les tables de pointeurs de `format_args!`. Absentes de l'image, elles laissent un **trou de zéros** à la bonne adresse. | `linker.ld` nomme `.got` et `.data.rel.ro` ; `objcopy -O binary` sans filtre |
| SSE interdit | `stage2.asm` ne mettait que `CR4.PAE` (`0x20`). Sans **`OSFXSR` (bit 9)**, toute instruction SSE lève `#UD`, or `x86_64-unknown-none` active SSE2 par défaut. | `or eax, 0x620` (PAE, OSFXSR, OSXMMEXCPT) |

**Fausse piste écartée :** la taille du noyau. Mesure réelle `21464 octets = 42 secteurs` sur 59 disponibles. Mesurer avant de conclure.

**Précaution conservée :** formater les entiers à la main via `serial::print_dec` et `Writer::write_dec`, comme `print_hex` le fait depuis le Jour 7. `build.sh` échoue si l'image dépasse 59 secteurs.

---

## Validation

### Arborescence

![Arborescence du projet avec timer.rs](./rendu_arborescence.png)

### Build et contrôle de taille

![Build termine et verification de la taille embarquable](./rendu_build_et_taille.png)

```
[4/4] Verification de la taille embarquable...
      Image kernel : 21464 octets = 42 secteurs (max 59)

OK - marge restante : 17 secteurs
```

### Boot

![Fenetre QEMU avec les cinq phases validees](./rendu_boot_qemu.png)

```
[4/5] Timer : PIT programme : diviseur 11931 -> 100 Hz (1 tick = 10 ms)
[5/5] IRQ   : Masque PIC1 = 0xFC -> IRQ0 + IRQ1 actives
Verification IRQ0 : OK - 1 tick(s) recus sans interaction clavier
```

### Les deux IRQ en parallèle

![Mot hello tape au clavier pendant que l'uptime defile](./rendu_avec_hello_et_action_sur_clavier.png)

```
[uptime] 21 s - 2100 ticks - 0 touches
Touche : h (t=21200 ms)
Touche : e (t=21860 ms)
[uptime] 22 s - 2200 ticks - 2 touches
Touche : l (t=22880 ms)
[uptime] 23 s - 2300 ticks - 3 touches
Touche : l (t=23030 ms)
Touche : o (t=23510 ms)
[uptime] 24 s - 2400 ticks - 5 touches
```

| Observation | Conclusion |
|---|---|
| Flux de ticks continu sur 41 s | `send_eoi(0)` fonctionne |
| Exactement 100 ticks/s, sans dérive | Diviseur juste, octets dans le bon ordre |
| `0 touches` pendant 21 s | Le noyau avance sans le clavier |
| Frappes intercalées entre les ticks | IRQ0 et IRQ1 coexistent |
| `t=21200 ms` correspond à 2120 ticks | Horodatage cohérent |
| 5 `c` en 1 s tous captés | Buffer circulaire sans perte |

### Protocole, piège à connaître

`cargo run` utilise `-display none -serial stdio` : les frappes du terminal partent sur le **port série**, pas sur le clavier PS/2. Aucune IRQ1, compteur à zéro, et pas d'écran VGA à capturer.

`run_demo.sh` ouvre une **vraie fenêtre QEMU** (VGA et clavier PS/2) avec la série dans le terminal. C'est le mode pour les captures et la vidéo :

```bash
./run_demo.sh
```

**Critère de réussite :** l'uptime défile **sans toucher au clavier**, et le clavier reste fonctionnel en parallèle.

La boucle de vérification est **bornée** (`spins > 200_000_000`) : si le masque PIC ou l'EOI est faux, on affiche `ECHEC` au lieu de figer.

---

## Angle Cyber

| Mécanisme | Risque ou protection |
|---|---|
| Timer précis | Base des canaux auxiliaires temporels (Spectre, Meltdown, comparaisons crypto non constant-time) |
| Diviseur configurable | Fréquence trop haute = noyade du CPU dans les handlers = DoS |
| Deadlock par interruption | Vraie classe de bug noyau, exploitable en DoS si l'attaquant maîtrise le timing |
| Reprise de contrôle | Sans préemption, un process qui boucle gèle tout ; le timer est la condition nécessaire pour reprendre la main de force |
| PIT contre TSC | Comparer les deux horloges révèle l'émulation, technique anti-VM des malwares |

> Le temps est une primitive de sécurité. Il permet de reprendre le contrôle d'un processus malveillant (défense), mais donne aussi à un attaquant l'instrument de mesure dont il a besoin pour les attaques par canal auxiliaire (offense).

---

## Limites du Jour 16, côté bootloader

La chaîne de chargement du Jour 3 arrive en bout de course. Elle a tenu treize journées, elle ne tiendra pas les suivantes.

| # | Limite | Détail |
|---|---|---|
| 1 | **Plafond 59 secteurs** | `mov al, 60` donne 30 208 o utiles ; 21 464 o consommés, marge 8,5 Ko |
| 2 | **Frontière 64 Ko** | `0x8000 + 64 × 512 = 0x10000` : le BIOS interdit de la franchir, donc 64 secteurs maximum en un appel |
| 3 | **Adressage CHS** | `cl` code le secteur sur 6 bits, mur au secteur 63, obsolète depuis les années 1990 |
| 4 | **Copie de taille fictive** | `rep movsd` recopie 128 Ko alors que 30 Ko au plus sont lus : 98 Ko de RAM non initialisée |
| 5 | **Zéro gestion d'erreur** | `jc $` = gel silencieux, sans réessai ni code d'erreur |
| 6 | **A20 en méthode unique** | Fast A20 Gate (port `0x92`) seulement, pas de repli 8042 ni `INT 15h` |
| 7 | **Aucune vérification** | Rien ne compare taille écrite et taille chargée, la classe de bug qui a coûté du temps aujourd'hui |

### Ce que le Jour 17 corrigera

| Chantier | Bénéfice |
|---|---|
| Lecture **LBA** (`INT 13h AH=0x42` et Disk Address Packet) | Fin du CHS, accès linéaire |
| Chargement **multi-blocs** avec segment incrémenté | Sortie du plafond 64 Ko |
| **Taille réelle** écrite dans l'image et lue par `stage2` | Fin des 128 Ko fictifs |
| **Réessais et message d'erreur** | Un disque capricieux ne gèle plus la machine |
| A20 avec **méthodes de repli** | Fonctionne au-delà de QEMU |

> Cohérent avec l'angle sécurité du projet : le bootloader est le **premier maillon de la chaîne de confiance**. Un chargement sans vérification de taille ni de cohérence est exactement ce qu'un attaquant vise pour injecter du code avant que le noyau, et donc toutes ses protections, n'existe.

---

## Ce que ça débloque

- **Jour 17** : bootloader LBA, puis sauvegarde et restauration du contexte CPU
- **Jour 18** : `schedule()` appelé depuis le handler IRQ0, passage au **préemptif**
