# Rapport Intermédiaire : Développement du Boot Sector (Amorçage)

Dans le cadre du développement de notre système d'exploitation sécurisé, nous avons franchi la première étape critique permettant d'interagir directement avec le matériel : l'écriture du **Bootloader** (ou secteur d'amorçage).

Ce rapport intermédiaire détaille la conception, le fonctionnement et la compilation de cette brique fondamentale.

## 1. Pourquoi un Bootloader personnalisé ?

Lorsqu'un ordinateur démarre, le BIOS effectue ses vérifications matérielles (POST) puis cherche un périphérique amorçable. S'il trouve un disque dont le premier secteur (exactement 512 octets) se termine par la signature hexadécimale `0xAA55`, il charge ce secteur en mémoire à l'adresse physique `0x7C00` et lui cède le contrôle.

Pour notre projet, nous n'avons pas réinventé la roue : nous avons adapté un bootloader éducatif classique (largement documenté dans la littérature de développement d'OS) afin de comprendre précisément l'initialisation matérielle sans subir la complexité des bootloaders modernes (comme GRUB). Cela nous permet de conserver le contrôle total et d'auditer l'intégralité de notre chaîne de démarrage d'un point de vue sécuritaire.

## 2. Fonctionnement de notre Bootloader

Notre bootloader effectue trois missions principales en cascade :

1. **Initialisation de l'environnement 16 bits** : Préparation de la pile et remise à zéro des registres de base pour un environnement sain.
2. **Chargement du Noyau (Kernel)** : Utilisation de l'interruption matérielle du BIOS (`Int 0x13`) pour lire le reste de notre système d'exploitation depuis le disque (à partir du secteur 2) et le placer en mémoire à l'adresse `0x8000`.
3. **Transition vers le Mode Protégé (32 bits)** : Le mode réel (16 bits) est obsolète et peu sécurisé. Nous configurons une Table des Descripteurs Globaux (GDT), nous activons le mode protégé dans le processeur, puis nous effectuons un saut (jump) vers notre noyau à l'adresse `0x8000`.

### Le Code Source (`src/boot.asm`)

Voici le code en assembleur x86 que nous avons mis en place pour réaliser cela :

```nasm
BITS 16
ORG 0x7C00

start:
    cli
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7C00

    ; Afficher message de boot
    mov si, msg_boot
    call print16

    ; Charger le kernel depuis le disque
    ; On lit 32 secteurs depuis le secteur 2
    ; et on les place à l'adresse 0x8000
    mov ax, 0x0800       ; segment 0x0800 = adresse 0x8000
    mov es, ax
    xor bx, bx           ; offset 0

    mov ah, 0x02         ; fonction BIOS : lire secteurs
    mov al, 32           ; nombre de secteurs à lire
    mov ch, 0            ; cylindre 0
    mov cl, 2            ; secteur 2 (le kernel commence ici)
    mov dh, 0            ; tête 0
    int 0x13             ; appel BIOS disque

    ; Vérifier si la lecture a réussi
    jc disk_error

    mov si, msg_ok
    call print16

    ; Passer en mode protégé 32 bits
    lgdt [gdt_descriptor]
    mov eax, cr0
    or eax, 1
    mov cr0, eax
    jmp 0x08:protected_mode

disk_error:
    mov si, msg_err
    call print16
    hlt

print16:
    lodsb
    or al, al
    jz print16_done
    mov ah, 0x0E
    int 0x10
    jmp print16
print16_done:
    ret

msg_boot db 'Booting kernel...', 0x0D, 0x0A, 0
msg_ok   db 'Kernel loaded!', 0x0D, 0x0A, 0
msg_err  db 'Disk error!', 0x0D, 0x0A, 0

gdt_start:
    dq 0                     ; descripteur nul obligatoire
    dq 0x00CF9A000000FFFF    ; segment code 32 bits
    dq 0x00CF92000000FFFF    ; segment data 32 bits
gdt_end:

gdt_descriptor:
    dw gdt_end - gdt_start - 1
    dd gdt_start

BITS 32
protected_mode:
    ; Initialiser les segments en mode protégé
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov esp, 0x90000

    ; Sauter vers le kernel à 0x8000
    call 0x8000

    ; Si le kernel retourne (ne devrait pas arriver)
    hlt

times 510-($-$$) db 0
dw 0xAA55
```

## 3. Compilation du Bootloader

Pour transformer ce code source en un secteur d'amorçage brut exécutable par le matériel, nous utilisons l'assembleur logiciel `nasm`.

**Commande de compilation :**

```bash
nasm -f bin src/boot.asm -o boot.bin
```

Cette commande produit un fichier binaire plat de 512 octets exactement (`boot.bin`). Le système informatique primaire est maintenant préparé et prêt à accueillir son noyau, point de départ que nous détaillons dans le rapport complet du Jour 3.
