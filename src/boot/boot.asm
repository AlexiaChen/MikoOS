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

; BIOS screen interrupt service: int 0x10 
; https://wiki.osdev.org/Text_Mode_Cursor

; clear screen
; AH feature number = 0x06
; AL = Row number = 0x00
; BH = Row property = 0x07
; (CL,CH) = Left upper corner of window (X,Y)
; (DL,DH) = Right lower corner of window (X,Y) 
    mov ax, 0x0600
    mov bx, 0x0700
    mov cx, 0x0000    ; Left upper corner (0,0) CL = 0x00, CH = 0x00
    mov dx, 0x184f    ; Right lower corner (80,25) 0x18=24, 0x4f=79 total 80 characters, start from (0,0) to (79,24)
    int 0x10

; set cursor focus
    mov ax, 0x0200    ; AH = 0x02 set cursor location
    mov bx, 0x0000    ; BH page number, BH = 0x00
    mov dx, 0x0000    ; (DH,DL) = cursor location (column num, row num) DH = 0x00 DL = 0x00
    int 0x10

; display on screen: start booting.....
    mov ax, 0x1301    ; AH = 0x13 displaying string, AL is write mode AL = 0x01 like AL=0x00, but cursor will move to end of string
    mov bx, 0x000f    ; BH page number = 0x00, BL = 0x0f character feature and character color
    mov dx, 0x0000    ; (DL,DH) = cursor location (column num, row num)
    mov cx, 18        ; length of string
    push ax
    mov ax, ds
    mov es, ax
    pop ax
    mov bp, StartBootMessage  ; es:bp is begin address of string you want to display
    int 0x10

; reset floppy
; BIOS interrupt service: 0x13 reset floppy
; https://wiki.osdev.org/BIOS
    xor ah, ah    ; reset ah to 0x00
    xor dl, dl    ; reset dl to 0x00  DL is drive number 0x00-0x7f floppy 0x80-0xff is hard drive
    int 0x13
    
    jmp $         ; floppy I/O head move to default
; String
StartBootMessage:  db  "Miko OS start boot"

; fill zero byte until whole MBR sector, compiled end of address - section begin of address (0x7c00)
    times 510 - ($ - $$)   db   0

; end of MBR sector must be 0xaa55  [begin..... 0x55, 0xaa]
    dw 0xaa55   ; cause intel cpu is little-endian


