section .text
global reload_cs

reload_cs:
    ; rdi contient le sélecteur CS (convention d'appel System V AMD64)
    pop rsi                 ; récupère l'adresse de retour
    push rdi                ; push le sélecteur CS
    push rsi                ; push l'adresse de retour
    retfq                   ; far return → recharge CS
