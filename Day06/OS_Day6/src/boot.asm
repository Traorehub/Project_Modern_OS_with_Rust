BITS 16
ORG 0x7C00

start:
    cli
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7C00

    mov si, msg_boot
    call print16

    mov ax, 0x0800
    mov es, ax
    xor bx, bx

    mov ah, 0x02
    mov al, 32
    mov ch, 0
    mov cl, 2
    mov dh, 0
    int 0x13

    jc disk_error

    mov si, msg_ok
    call print16

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
    dq 0
    dq 0x00CF9A000000FFFF
    dq 0x00CF92000000FFFF
gdt_end:

gdt_descriptor:
    dw gdt_end - gdt_start - 1
    dd gdt_start

BITS 32
protected_mode:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov esp, 0x90000

    call 0x8000

    hlt

times 510-($-$$) db 0
dw 0xAA55
