BITS 32
ORG 0x8000

mode32:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov fs, ax
    mov gs, ax
    mov esp, 0x90000

    ; Copier kernel de 0x8200 vers 0x200000
    ; (0x8200 = 0x8000 + 512 = juste après ce secteur)
    mov esi, 0x8200
    mov edi, 0x200000
    mov ecx, 0x8000
    rep movsd

    ; Tables de pages
    mov edi, 0x1000
    xor eax, eax
    mov ecx, 0xC00
    rep stosd

    mov dword [0x1000], 0x2000 | 0x3
    mov dword [0x2000], 0x3000 | 0x3
    mov dword [0x3000], 0x000083
    mov dword [0x3008], 0x200083

    ; PAE
    mov eax, cr4
    or eax, 0x20
    mov cr4, eax

    ; CR3
    mov eax, 0x1000
    mov cr3, eax

    ; EFER.LME
    mov ecx, 0xC0000080
    rdmsr
    or eax, 0x100
    wrmsr

    ; Paging ON
    mov eax, cr0
    or eax, 0x80000000
    mov cr0, eax

    lgdt [gdt64_ptr]
    jmp 0x08:mode64

gdt64:
    dq 0
    dq 0x00AF9A000000FFFF
    dq 0x00AF92000000FFFF
gdt64_ptr:
    dw 23
    dd gdt64

BITS 64
mode64:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov rsp, 0x90000
    mov rax, 0x200000
    call rax
    hlt
    jmp $

times 512-($-$$) db 0
