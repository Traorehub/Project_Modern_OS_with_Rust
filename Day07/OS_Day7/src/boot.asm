BITS 16
ORG 0x7C00

start:
    cli
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7C00

    ; A20
    in al, 0x92
    or al, 2
    out 0x92, al

    ; Charger 60 secteurs à 0x8000
    ; Le payload contient : [code32 (512o)] + [kernel]
    mov ax, 0x0800
    mov es, ax
    xor bx, bx
    mov ah, 0x02
    mov al, 60
    mov ch, 0
    mov cl, 2
    mov dh, 0
    int 0x13
    jc $

    lgdt [gdt_ptr]
    mov eax, cr0
    or eax, 1
    mov cr0, eax
    jmp 0x08:0x8000

gdt:
    dq 0
    dq 0x00CF9A000000FFFF
    dq 0x00CF92000000FFFF
gdt_ptr:
    dw 23
    dd gdt

times 510-($-$$) db 0
dw 0xAA55
