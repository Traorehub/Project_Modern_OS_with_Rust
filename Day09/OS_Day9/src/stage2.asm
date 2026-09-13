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

    mov esi, 0x8200
    mov edi, 0x200000
    mov ecx, 0x8000
    rep movsd

    ; Tables de pages à 0x70000
    mov edi, 0x70000
    xor eax, eax
    mov ecx, 0x1800
    rep stosd

    ; PML4[0] → PDPT 0x71000
    mov dword [0x70000], 0x71000 | 0x3
    ; PDPT[0] → PD 0x72000
    mov dword [0x71000], 0x72000 | 0x3
    ; Mapper 16 huge pages de 2Mo = 32Mo
    mov dword [0x72000], 0x000083
    mov dword [0x72008], 0x200083
    mov dword [0x72010], 0x400083
    mov dword [0x72018], 0x600083
    mov dword [0x72020], 0x800083
    mov dword [0x72028], 0xa00083
    mov dword [0x72030], 0xc00083
    mov dword [0x72038], 0xe00083
    mov dword [0x72040], 0x1000083
    mov dword [0x72048], 0x1200083
    mov dword [0x72050], 0x1400083
    mov dword [0x72058], 0x1600083
    mov dword [0x72060], 0x1800083
    mov dword [0x72068], 0x1a00083
    mov dword [0x72070], 0x1c00083
    mov dword [0x72078], 0x1e00083

    mov eax, cr4
    or eax, 0x20
    mov cr4, eax

    mov eax, 0x70000
    mov cr3, eax

    mov ecx, 0xC0000080
    rdmsr
    or eax, 0x100
    wrmsr

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
