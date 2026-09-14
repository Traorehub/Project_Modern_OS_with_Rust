; Jour 17 : boot 16 bits, A20 avec repli, lecture LBA multi-blocs.
; Le BIOS charge ce secteur a 0x7C00. DL contient le numero de lecteur.
;
; Image :
;   LBA 0 : ce MBR
;   LBA 1 : stage2 (1 secteur, 0x8000)
;   LBA 2+: kernel, charge a 0x10000 par paquets de 64 secteurs
;
; Offsets fixes (patches par patch_boot_size.sh) :
;   500 : kernel_bytes    (u32 LE)
;   504 : kernel_sectors  (u16 LE)

BITS 16
ORG 0x7C00

start:
    jmp short real_start
    nop

real_start:
    cli
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7C00
    mov [boot_drive], dl

    call enable_a20
    call check_lba
    call load_all

    lgdt [gdt_ptr]
    mov eax, cr0
    or eax, 1
    mov cr0, eax
    jmp 0x08:0x8000

; ---------------------------------------------------------------------------
; A20 : Fast Gate, puis INT 15h, puis 8042. On verifie apres chaque methode.
; ---------------------------------------------------------------------------
enable_a20:
    call a20_fast
    call a20_is_on
    jc .ok

    mov ax, 0x2401
    int 0x15
    call a20_is_on
    jc .ok

    call a20_8042
    call a20_is_on
    jc .ok

    mov si, msg_a20
    jmp fatal

.ok:
    ret

a20_fast:
    in al, 0x92
    or al, 2
    and al, 0xFE
    out 0x92, al
    ret

; CF=1 si A20 ouverte : 0000:0500 et FFFF:0510 sont des adresses distinctes.
a20_is_on:
    push ds
    push es
    push si
    push di
    xor ax, ax
    mov es, ax
    not ax
    mov ds, ax
    mov di, 0x0500
    mov si, 0x0510
    mov al, [es:di]
    mov ah, [ds:si]
    mov byte [es:di], 0x00
    mov byte [ds:si], 0xFF
    cmp byte [es:di], 0xFF
    mov [es:di], al
    mov [ds:si], ah
    pop di
    pop si
    pop es
    pop ds
    je .off
    stc
    ret
.off:
    clc
    ret

a20_8042:
    call .wait_in
    mov al, 0xAD
    out 0x64, al
    call .wait_in
    mov al, 0xD0
    out 0x64, al
    call .wait_out
    in al, 0x60
    push ax
    call .wait_in
    mov al, 0xD1
    out 0x64, al
    call .wait_in
    pop ax
    or al, 2
    out 0x60, al
    call .wait_in
    mov al, 0xAE
    out 0x64, al
    ret
.wait_in:
    in al, 0x64
    test al, 2
    jnz .wait_in
    ret
.wait_out:
    in al, 0x64
    test al, 1
    jz .wait_out
    ret

; ---------------------------------------------------------------------------
; INT 13h AH=0x41 : le BIOS doit annoncer les extensions LBA.
; ---------------------------------------------------------------------------
check_lba:
    mov ah, 0x41
    mov bx, 0x55AA
    mov dl, [boot_drive]
    int 0x13
    jc .fail
    cmp bx, 0xAA55
    jne .fail
    ret
.fail:
    mov si, msg_lba
    jmp fatal

; ---------------------------------------------------------------------------
; Charge stage2 (LBA 1 -> 0x8000) puis le kernel (LBA 2+ -> 0x10000).
; Paquets de 64 secteurs (32 Ko) : jamais de franchissement de 64 Ko.
; ---------------------------------------------------------------------------
load_all:
    mov word [dap_count], 1
    mov word [dap_off], 0
    mov word [dap_seg], 0x0800
    mov dword [dap_lba], 1
    mov dword [dap_lba+4], 0
    call read_lba

    mov cx, [kernel_sectors]
    test cx, cx
    jz .done
    mov dword [dap_lba], 2
    mov word [dap_seg], 0x1000
    mov word [dap_off], 0

.next:
    test cx, cx
    jz .done
    mov ax, 64
    cmp cx, ax
    jae .use_ax
    mov ax, cx
.use_ax:
    mov [dap_count], ax
    call read_lba
    sub cx, ax
    add [dap_lba], ax
    shl ax, 5
    add [dap_seg], ax
    jmp .next

.done:
    ret

read_lba:
    pusha
    mov di, 3
.retry:
    mov dl, [boot_drive]
    mov ah, 0x42
    mov si, dap
    int 0x13
    jnc .ok
    xor ah, ah
    mov dl, [boot_drive]
    int 0x13
    dec di
    jnz .retry
    mov si, msg_disk
    jmp fatal
.ok:
    popa
    ret

fatal:
    lodsb
    test al, al
    jz .hang
    mov ah, 0x0E
    mov bx, 0x0007
    int 0x10
    jmp fatal
.hang:
    cli
    hlt
    jmp .hang

boot_drive: db 0

dap:
    db 16
    db 0
dap_count: dw 0
dap_off:   dw 0
dap_seg:   dw 0
dap_lba:   dq 0

msg_a20:  db "A20", 0
msg_lba:  db "LBA", 0
msg_disk: db "DISK", 0

gdt:
    dq 0
    dq 0x00CF9A000000FFFF
    dq 0x00CF92000000FFFF
gdt_ptr:
    dw 23
    dd gdt

    times 500-($-$$) db 0
kernel_bytes:   dd 0
kernel_sectors: dw 0
    times 510-($-$$) db 0
    dw 0xAA55
