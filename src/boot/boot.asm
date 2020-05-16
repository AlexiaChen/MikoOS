; Origin Address, cause BIOS will load the MBR to memory address 0x7c00 
; https://www.glamenv-septzen.net/en/view/6
    org 0x7c00

; Stack Base Address
    BaseOfStack equ 0x7c00

; Label Start, Setg Segment base address to ds,es,ss,fs register and set stack pointer register sp
_Start:
    mov ax, cs
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov fs, ax
    mov sp, BaseOfStack

; clear screen
; AH feature number = 0x06
; AL = Row number = 0x00
; BH = Row property = 0x07
; (CL,CH) = Left upper corner of window (X,Y)
; (DL,DH) = Right lower corner of window (X,Y)
; BIOS screen interrupt service: int 0x10 
; https://wiki.osdev.org/Text_Mode_Cursor 
    mov ax, 0x0600
    mov bx, 0x0700
    mov cx, 0         ; Left upper corner (0,0)
    mov dx, 0x184f    ; Right lower corner (80,25) 0x18=24, 0x4f=79 total 80 characters, start from (0,0) to (79,24)
    int 0x10

; set focus
    mov ax, 0x0200    ; AH = 0x02 set cursor location
    mov bx, 0x0000    ; BH page number, BH = 0x00
    mov dx, 0x0000    ; (DH,DL) = cursor location (column num, row number) DH = 0x00 DL = 0x00
    int 0x10

; display on screen: start booting.....


