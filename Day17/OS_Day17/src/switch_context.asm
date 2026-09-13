; Jour 17 : commutation de contexte cooperative.
; rdi = ancien ThreadContext*, rsi = nouveau ThreadContext*
;
; Offsets (voir context.rs) :
;   0 r15, 8 r14, 16 r13, 24 r12, 32 rbx, 40 rbp
;   48 rsp, 56 rip, 64 rflags
section .text
global switch_context

switch_context:
    mov [rdi + 0], r15
    mov [rdi + 8], r14
    mov [rdi + 16], r13
    mov [rdi + 24], r12
    mov [rdi + 32], rbx
    mov [rdi + 40], rbp

    lea rax, [rsp + 8]
    mov [rdi + 48], rax
    mov rax, [rsp]
    mov [rdi + 56], rax
    pushfq
    pop rax
    mov [rdi + 64], rax

    mov r15, [rsi + 0]
    mov r14, [rsi + 8]
    mov r13, [rsi + 16]
    mov r12, [rsi + 24]
    mov rbx, [rsi + 32]
    mov rbp, [rsi + 40]

    mov rax, [rsi + 64]
    push rax
    popfq
    mov rsp, [rsi + 48]
    jmp qword [rsi + 56]
