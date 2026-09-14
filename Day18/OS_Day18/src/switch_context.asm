; Jour 18 : commutation complete (callee-saved + FXSAVE).
; rdi = ancien ThreadContext*, rsi = nouveau ThreadContext*
;
; Offsets (voir context.rs, struct alignee 16 octets) :
;   0   fx[512]
;   512 r15, 520 r14, 528 r13, 536 r12, 544 rbx, 552 rbp
;   560 rsp, 568 rip, 576 rflags
section .text
global switch_context

switch_context:
    fxsave64 [rdi]

    mov [rdi + 512], r15
    mov [rdi + 520], r14
    mov [rdi + 528], r13
    mov [rdi + 536], r12
    mov [rdi + 544], rbx
    mov [rdi + 552], rbp

    lea rax, [rsp + 8]
    mov [rdi + 560], rax
    mov rax, [rsp]
    mov [rdi + 568], rax
    pushfq
    pop rax
    mov [rdi + 576], rax

    fxrstor64 [rsi]

    mov r15, [rsi + 512]
    mov r14, [rsi + 520]
    mov r13, [rsi + 528]
    mov r12, [rsi + 536]
    mov rbx, [rsi + 544]
    mov rbp, [rsi + 552]

    mov rax, [rsi + 576]
    push rax
    popfq
    mov rsp, [rsi + 560]
    jmp qword [rsi + 568]
