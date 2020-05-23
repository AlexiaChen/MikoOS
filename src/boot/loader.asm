org 0x10000 ; BaseOfLoader << 4

    mov ax, cs
    mov ds, ax
    mov es, ax
    mov ax, 0x00
    mov ss, ax
    mov sp, 0x7c00

; =========== display on screen : Start Loader

    mov ax, 0x1301
    mov bx, 0x000f
    mov dx, 0x0200
    mov cx, 20   ; length of StartLoaderMessage
    push ax
    mov ax, ds
    mov es, ax
    pop ax
    mov bp, StartLoaderMessage
    int 0x10

    jmp $

    ; ========== display message

    StartLoaderMessage: db "Start Miko OS Loader" 
