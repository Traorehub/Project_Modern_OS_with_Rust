# Rapport Jour 3 : Le Noyau, le Panic Handler & L'Intégration Finale

Suite à la création de notre bootloader (présenté dans le rapport intermédiaire), nous nous sommes concentrés sur le cœur de notre système d'exploitation : le noyau écrit en Rust, son mécanisme de sécurité face aux pannes (Panic Handler), et l'intégration finale (édition des liens) avec le secteur d'amorçage.

![Concept Panic & Bootloader](day3_concept.png)

## 1. Le Noyau et le Panic Handler

### Pourquoi la gestion des pannes est critique en Cyber ?
Dans un OS standard (Windows, Linux), si un programme subit une erreur fatale (`panic!`), le noyau hôte l'intercepte, nettoie la mémoire et ferme le processus proprement. Dans notre cas de développement "bare-metal", nous écrivons ce noyau, il n'y a donc **aucun système hôte** pour nous secourir.

Sans Panic Handler configuré, une erreur forcerait le processeur à continuer d'exécuter des instructions en cascade sur des adresses mémoires corrompues. Sur le plan de la cybersécurité, ce comportement indéfini (Undefined Behavior) crée une surface d'attaque critique souvent exploitée pour l'exécution de code arbitraire.

### Notre Mécanisme de Sécurité
Pour pallier ce risque, nous avons implémenté une isolation stricte :
* **Redirection des Logs (Forensics)** : Les erreurs détaillées sont envoyées de manière "hors-bande" via le port série matériel (COM1 - adresse `0x3F8`).
* **Hardware Halt** : Dès qu'un panic survient, le processeur exécute l'instruction hardware `hlt` en boucle infinie. Cela stoppe net toute activité et ferme instantanément la fenêtre d'exploitabilité.

### Code du Noyau (`src/main.rs`)

```rust
#![no_std]
#![no_main]

use core::panic::PanicInfo;

const COM1: u16 = 0x3F8;

unsafe fn write_serial(byte: u8) {
    unsafe {
        core::arch::asm!("out dx, al", in("dx") COM1, in("al") byte);
    }
}

unsafe fn print_serial(s: &str) {
    for byte in s.bytes() {
        unsafe { write_serial(byte); }
    }
}

unsafe fn halt() -> ! {
    loop {
        unsafe { core::arch::asm!("hlt"); }
    }
}

// Nous forçons cette fonction pour qu'elle soit chargée tout au début de la mémoire
#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
pub extern "C" fn _start() -> ! {
    unsafe {
        print_serial("OS starting...\n");
        // Déclenchement volontaire pour prouver le fonctionnement de la sécurité
        panic!("Test panic : quelque chose a mal tourne !");
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    unsafe {
        print_serial("\n=== KERNEL PANIC ===\n");
        if let Some(msg) = info.message().as_str() {
            print_serial("Raison  : ");
            print_serial(msg); 
            print_serial("\n");
        }
        if let Some(location) = info.location() {
            print_serial("Fichier : ");
            print_serial(location.file()); 
            print_serial("\n");
        }
        print_serial("Systeme arrete.\n====================\n");
        halt();
    }
}
```

## 2. Configuration et Alignement (Liaison)

L'intégration d'un bootloader ASM avec un noyau Rust repose sur deux paramètres fondamentaux :

### Cible de Compilation 32 bits
Notre bootloader passe le processeur en **32 bits**. L'environnement Rust doit donc compiler un noyau compatible. Nous utilisons pour cela une cible sur-mesure (`i686-unknown-none.json`) forçant Rust à produire un binaire strictement 32 bits bare-metal.

### L'Alignement en Mémoire (Script Linker)
Le bootloader saute invariablement à l'adresse fixe `0x8000`. La fonction `_start` de Rust doit y être disposée très exactement.
Nous avons forcé ce placement mathématique grâce à l'attribut `#[unsafe(link_section = ".text.entry")]` dans le code (vu plus haut), exploité par ce script de liaison :

**Fichier de liaison (`linker.ld`)** :
```ld
ENTRY(_start)
SECTIONS {
    . = 0x8000;
    .text : {
        *(.text.entry)   /* Garanti d'être placé et exécuté en premier ! */
        *(.text)
        *(.text.*)
    }
    .rodata : { *(.rodata) *(.rodata.*) }
    .data : { *(.data) *(.data.*) }
    .bss : { *(.bss) *(.bss.*) }
}
```

## 3. Commandes d'Assemblage Final

L'assemblage final consiste à compiler le noyau, extraire son code machine pur, puis concaténer le bootloader (`boot.bin`) et ce noyau brut (`kernel.bin`) dans un seul fichier d'image disque (`os.img`).

**Voici les commandes complètes d'exécution :**

```bash
# 1. Compilation du noyau Rust 32 bits via notre fichier cible JSON
cargo build -Z json-target-spec

# 2. Extraction du code machine brut (flat binary)
objcopy -O binary target/i686-unknown-none/debug/kernel kernel.bin

# 3. Création et assemblage de l'image disque finale
dd if=/dev/zero of=os.img bs=512 count=2048
dd if=boot.bin of=os.img conv=notrunc
dd if=kernel.bin of=os.img bs=512 seek=1 conv=notrunc

# 4. Exécution sous QEMU (avec l'interface série branchée sur la console hôte)
killall qemu-system-x86_64 2>/dev/null
qemu-system-x86_64 -drive format=raw,file=os.img -serial stdio -no-reboot -no-shutdown -display none
```

### Résultat de l'Exécution

![Résultat QEMU et Terminal](image.png)

**Pourquoi le message s'affiche-t-il dans le terminal hôte et non dans la fenêtre QEMU ?**
Dans notre architecture bare-metal, nous n'avons pas encore développé de pilote pour l'affichage VGA. À la place, nous avons configuré le noyau pour envoyer ses textes directement au port de communication série matériel (`COM1 - 0x3F8`), une méthode couramment utilisée pour le débogage (out-of-band logging).
Lors de la commande d'exécution, l'option ` -serial stdio ` demande à QEMU de capturer les signaux de ce port série et de les afficher directement dans notre terminal Kali. C'est pour cette raison que la fenêtre QEMU reste noire (aucun pilote vidéo actif) tandis que notre fameux `=== KERNEL PANIC ===` s'affiche proprement dans la console hôte.

## 🛡️ Résumé - Analyse d'Angle Cyber

| Mécanisme | Vecteur d'Attaque | Mitigation / Protection |
| :--- | :--- | :--- |
| **Boot Sector** | Menace de **Bootkit** et altération MBR. | (Perspectives) Signature de code au boot (type UEFI Secure Boot). |
| **Transition 16 -> 32 bits** | Détournement avant l'isolation mémoire. | Configuration drastique de la GDT pour isoler le segment kernel. |
| **Panic Handler** | Comportements indéfinis propices aux exploits. | `halt()` immédiat du système pour refermer toute brèche temporelle d'exploitation. |
| **Port Série Debug** | Fuite d'informations sensibles (Covert Channel). | Peut être retiré via condition de compilation en environnement de production (Release). |
