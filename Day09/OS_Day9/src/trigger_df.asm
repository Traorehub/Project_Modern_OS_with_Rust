section .text
global trigger_double_fault
global double_fault_handler_asm

trigger_double_fault:
    mov rax, 0x5000000
    mov qword [rax], 0
    ret

double_fault_handler_asm:
    ; Ligne 24 (dernière) = offset (24 * 80) * 2 = 3840 = 0xF00
    ; 0xb8000 + 0xF00 = 0xb8F00
    mov byte [0xb8F00], '['
    mov byte [0xb8F01], 0x4F
    mov byte [0xb8F02], 'D'
    mov byte [0xb8F03], 0x4F
    mov byte [0xb8F04], 'O'
    mov byte [0xb8F05], 0x4F
    mov byte [0xb8F06], 'U'
    mov byte [0xb8F07], 0x4F
    mov byte [0xb8F08], 'B'
    mov byte [0xb8F09], 0x4F
    mov byte [0xb8F0A], 'L'
    mov byte [0xb8F0B], 0x4F
    mov byte [0xb8F0C], 'E'
    mov byte [0xb8F0D], 0x4F
    mov byte [0xb8F0E], ' '
    mov byte [0xb8F0F], 0x4F
    mov byte [0xb8F10], 'F'
    mov byte [0xb8F11], 0x4F
    mov byte [0xb8F12], 'A'
    mov byte [0xb8F13], 0x4F
    mov byte [0xb8F14], 'U'
    mov byte [0xb8F15], 0x4F
    mov byte [0xb8F16], 'L'
    mov byte [0xb8F17], 0x4F
    mov byte [0xb8F18], 'T'
    mov byte [0xb8F19], 0x4F
    mov byte [0xb8F1A], ']'
    mov byte [0xb8F1B], 0x4F
    mov byte [0xb8F1C], ' '
    mov byte [0xb8F1D], 0x4F
    mov byte [0xb8F1E], 'I'
    mov byte [0xb8F1F], 0x4F
    mov byte [0xb8F20], 'S'
    mov byte [0xb8F21], 0x4F
    mov byte [0xb8F22], 'T'
    mov byte [0xb8F23], 0x4F
    mov byte [0xb8F24], ' '
    mov byte [0xb8F25], 0x4F
    mov byte [0xb8F26], 'O'
    mov byte [0xb8F27], 0x4F
    mov byte [0xb8F28], 'K'
    mov byte [0xb8F29], 0x4F
    mov byte [0xb8F2A], '!'
    mov byte [0xb8F2B], 0x4F

    mov dx, 0x3F8
    mov al, 0x0A
    out dx, al
    mov al, '['
    out dx, al
    mov al, 'D'
    out dx, al
    mov al, 'F'
    out dx, al
    mov al, ' '
    out dx, al
    mov al, 'I'
    out dx, al
    mov al, 'S'
    out dx, al
    mov al, 'T'
    out dx, al
    mov al, ' '
    out dx, al
    mov al, 'O'
    out dx, al
    mov al, 'K'
    out dx, al
    mov al, '!'
    out dx, al
    mov al, 0x0A
    out dx, al

.halt:
    hlt
    jmp .halt
