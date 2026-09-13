section .text
global trigger_double_fault
global double_fault_handler_asm

; Déclenche un Double Fault : accès mémoire non mappé → Page Fault
; Page Fault handler corrompt RSP → Double Fault → handler ASM
trigger_double_fault:
    mov rax, 0x5000000
    mov qword [rax], 0
    ret

; Handler Double Fault pur ASM — zéro prologue Rust
; Écrit directement sur COM1 et VGA puis hlt
double_fault_handler_asm:
    ; Écrire "DF" sur VGA à (0,0) en blanc sur rouge
    mov byte [0xb8000], 'D'
    mov byte [0xb8001], 0x4F
    mov byte [0xb8002], 'F'
    mov byte [0xb8003], 0x4F
    mov byte [0xb8004], '!'
    mov byte [0xb8005], 0x4F

    ; Écrire sur COM1
    mov dx, 0x3F8
    mov al, 0x0A      ; newline
    out dx, al
    mov al, 'D'
    out dx, al
    mov al, 'F'
    out dx, al
    mov al, '!'
    out dx, al
    mov al, 0x0A
    out dx, al

.halt:
    hlt
    jmp .halt
