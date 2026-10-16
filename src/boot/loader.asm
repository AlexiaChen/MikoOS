org 0x10000 ; BaseOfLoader << 4
    jmp _Start

%include "src/boot/fat12.inc"

BaseOfKernelFile equ 0x00
OffsetOfKernelFile equ 0x100000 ; 1MB offset

BaseTempOfKernelAddress equ 0x00 
OffsetTempOfKernelFile equ 0x7E00 ; Temp space address for reading kernel file

; Rust 交接协议：32 个 20 字节 E820 条目，数量单独存放。
; 旧 0x7e00 缓冲区可能与 0x8000 VBE 信息重叠，改为不冲突的低端内存。
MemoryStructBufferAddress equ 0x5000
MemoryStructCountAddress equ 0x4ff0
MemoryStructMaxEntries equ 32

[SECTION gdt]

; 第 6 章：保护模式的段寄存器保存选择子，CPU 从 GDT 取出隐藏的 base/limit。
; 这里 CODE32/DATA32 都是 base=0、limit=4 GiB-1 的平坦段（G=1、D/B=1）。
; “平坦”表示段变换不再平移地址，并不等于已经启用分页或拥有 4 GiB RAM。
LABEL_GDT:		dd	0,0
LABEL_DESC_CODE32:	dd	0x0000FFFF,0x00CF9A00
LABEL_DESC_DATA32:	dd	0x0000FFFF,0x00CF9200

GdtLen	equ	$ - LABEL_GDT
GdtPtr	dw	GdtLen - 1
	dd	LABEL_GDT

SelectorCode32	equ	LABEL_DESC_CODE32 - LABEL_GDT
SelectorData32	equ	LABEL_DESC_DATA32 - LABEL_GDT

[SECTION gdt64]

; 64 位代码段 L=1、D=0。IA-32e 激活后还必须加载这样的 CS 才执行 64 位代码；
; NASM 的 BITS 只改变生成的指令编码，本身不修改 CPU 模式。
LABEL_GDT64:		dq	0x0000000000000000
LABEL_DESC_CODE64:	dq	0x0020980000000000
LABEL_DESC_DATA64:	dq	0x0000920000000000

GdtLen64	equ	$ - LABEL_GDT64
GdtPtr64	dw	GdtLen64 - 1
		dd	LABEL_GDT64

SelectorCode64	equ	LABEL_DESC_CODE64 - LABEL_GDT64
SelectorData64	equ	LABEL_DESC_DATA64 - LABEL_GDT64

    
[SECTION .s16]
[BITS 16]    ; specify nasm use 16-bits mode

_Start:
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

; A20 是地址线门控：打开后 bit20 不再强制为 0，跨过 1 MiB 的访问不会回绕。
; 它不改变段 limit，也不启用分页；访问更高地址还需要下面的段缓存设置。

    push ax
    in al, 0x92
    or al, 00000010b
    out 0x92, al
    pop ax

    cli ; close external interrupt

    db 0x66
    lgdt [GdtPtr] ; load information of protection mode

    mov eax, cr0
    or eax, 1 ; set bit 0 to 1 for cr0 register to open protection mode
    mov cr0, eax

; big/unreal mode：PE=1 时为 FS 加载 base=0、4 GiB limit 的描述符，再清 PE。
; 回到实模式后仍借用 FS 的隐藏缓存，用 32 位偏移向物理 1 MiB 复制内核。
; 此时 CS 仍走 16 位引导流程，可以继续使用 BIOS；不能重新加载 FS 抹掉缓存。
; 这是现有 Loader 的阶段性做法，更完整的引导可靠性在第 7 章继续处理。
    mov ax, SelectorData32
    mov fs, ax
    mov eax, cr0
    and al, 11111110b
    mov cr0, eax

    sti

    ;=======	reset floppy

	xor	ah,	ah
	xor	dl,	dl
	int	0x13

;=======	search kernel.bin
	mov	word	[SectorNo],	SectorNumOfRootDirStart

_Search_In_Root_Dir_Begin:

	cmp	word	[RootDirSizeForLoop],	0
	jz	_No_KernelBin
	dec	word	[RootDirSizeForLoop]	
	mov	ax,	0x00
	mov	es,	ax
	mov	bx,	0x8000
	mov	ax,	[SectorNo]
	mov	cl,	1
	call	_ReadOneSectorFunc
	mov	si,	KernelFileName
	mov	di,	0x8000
	cld
	mov	dx,	0x10
	
_Search_For_KernelBin:

	cmp	dx,	0
	jz	_Goto_Next_Sector_In_Root_Dir
	dec	dx
	mov	cx,	11

_Cmp_FileName:

	cmp	cx,	0
	jz	_FileName_Found
	dec	cx
	lodsb	
	cmp	al,	byte	[es:di]
	jz	_Go_On
	jmp	_Different

_Go_On:
	
	inc	di
	jmp	_Cmp_FileName

_Different:

	and	di,	0x0FFE0
	add	di,	0x20
	mov	si,	KernelFileName
	jmp	_Search_For_KernelBin

_Goto_Next_Sector_In_Root_Dir:
	
	add	word	[SectorNo],	1
	jmp	_Search_In_Root_Dir_Begin
	
;=======	display on screen : ERROR:No KERNEL Found

_No_KernelBin:

	mov	ax,	0x1301
	mov	bx,	0x008C
	mov	dx,	0x0300		;row 3
	mov	cx,	21 ; length of NoKernelMessage
	push	ax
	mov	ax,	ds
	mov	es,	ax
	pop	ax
	mov	bp,	NoKernelMessage
	int	0x10
	jmp	$

;=======	found loader.bin name in root director struct

_FileName_Found:
	mov	ax,	RootDirSectors
	and	di,	0x0FFE0
	add	di,	0x01A
	mov	cx,	word	[es:di]
	push	cx
	add	cx,	ax
	add	cx,	SectorBalance
	mov	eax,	BaseTempOfKernelAddress	;BaseOfKernelFile
	mov	es,	eax
	mov	bx,	OffsetTempOfKernelFile	;OffsetOfKernelFile
	mov	ax,	cx

_Go_On_Loading_File:
	push	ax
	push	bx
	mov	ah,	0x0E
	mov	al,	'.'
	mov	bl,	0x0F
	int	0x10
	pop	bx
	pop	ax

	mov	cl,	1
	call	_ReadOneSectorFunc
	pop	ax

;;;;;;;;;;;;;;;;;;;;;;;	
	push	cx
	push	eax
	push	fs
	push	edi
	push	ds
	push	esi

	mov	cx,	0x200
	mov	ax,	BaseOfKernelFile
	mov	fs,	ax
	mov	edi,	dword	[OffsetOfKernelFileCount]

	mov	ax,	BaseTempOfKernelAddress
	mov	ds,	ax
	mov	esi,	OffsetTempOfKernelFile

_Mov_Kernel:
    mov	al,	byte	[ds:esi]
	mov	byte	[fs:edi],	al

	inc	esi
	inc	edi

	loop _Mov_Kernel

	mov	eax,	0x1000
	mov	ds,	eax

	mov	dword	[OffsetOfKernelFileCount],	edi

	pop	esi
	pop	ds
	pop	edi
	pop	fs
	pop	eax
	pop	cx

;;;;;;;;;;;;;;;;;;;;;;;	

	call _GetFATEntryFunc
	cmp	ax,	0FFFh
	jz	_File_Loaded
	push	ax
	mov	dx,	RootDirSectors
	add	ax,	dx
	add	ax,	SectorBalance

	jmp	_Go_On_Loading_File

_File_Loaded:
	mov	ax, 0x0B800 ; base address of display character
	mov	gs, ax
	mov	ah, 0x0F				; 0000: block background   1111: white text
	mov	al, 'G'
	mov	[gs:((80 * 0 + 39) * 2)], ax	; row 0 colum 39 on screen
    
KillMotor:
	; 0x03f2 IO port is control of floppy
	push	dx
	mov	dx,	0x03F2
	mov	al,	0	; close all dirve
	out	dx,	al
	pop	dx

;=======	get memory address size type

	mov	ax,	0x1301
	mov	bx,	0x000F
	mov	dx,	0x0400		;row 4
	mov	cx,	24 ; length of StartGetMemStructMessage
	push	ax
	mov	ax,	ds
	mov	es,	ax
	pop	ax
	mov	bp,	StartGetMemStructMessage
	int	0x10

	mov	ebx,	0
	mov	ax,	0x00
	mov	es,	ax
	mov	di,	MemoryStructBufferAddress
	mov word [es:MemoryStructCountAddress], 0

_Get_Mem_Struct:
    cmp word [es:MemoryStructCountAddress], MemoryStructMaxEntries
    jae _Get_Mem_Fail ; 容量不足时停止，不能越界或静默截断内存图
	; E820 memory map
	mov	eax,	0x0E820 ; 
	mov	ecx,	20 ; buffer szie must >= 20
	mov	edx,	0x534D4150 ; signature "SMAP" used by the BIOS to verify the caller is requesting the system map info to be returned by
	int	15h
	jc	_Get_Mem_Fail
    cmp eax, 0x534d4150
    jne _Get_Mem_Fail
    cmp ecx, 20
    jb _Get_Mem_Fail
    inc word [es:MemoryStructCountAddress]
	add	di,	20

	cmp	ebx,	0; returned previous memory map if first call it return 0
	jne	_Get_Mem_Struct
	jmp	_Get_Mem_OK

_Get_Mem_Fail:

	mov	ax,	0x1301
	mov	bx,	0x008C
	mov	dx,	0x0500		;row 5
	mov	cx,	23 ; length of GetMemStructErrMessage
	push	ax
	mov	ax,	ds
	mov	es,	ax
	pop	ax
	mov	bp,	GetMemStructErrMessage
	int	0x10
	jmp	$

_Get_Mem_OK:
	
	mov	ax,	0x1301
	mov	bx,	0x000F
	mov	dx,	0x0600		;row 6
	mov	cx,	29 ; length of GetMemStructOKMessage
	push	ax
	mov	ax,	ds
	mov	es,	ax
	pop	ax
	mov	bp,	GetMemStructOKMessage
	int	0x10

;=======	get SVGA information

	mov	ax,	0x1301
	mov	bx,	0x000F
	mov	dx,	0x0800		;row 8
	mov	cx,	23
	push	ax
	mov	ax,	ds
	mov	es,	ax
	pop	ax
	mov	bp,	StartGetSVGAVBEInfoMessage
	int	0x10

	mov	ax,	0x00
	mov	es,	ax
	mov	di,	0x8000
	mov	ax,	0x4F00

	int	0x10

	cmp	ax,	0x004F

	jz	.KO
	
;=======	Fail

	mov	ax,	1301h
	mov	bx,	008Ch
	mov	dx,	0900h		;row 9
	mov	cx,	23
	push	ax
	mov	ax,	ds
	mov	es,	ax
	pop	ax
	mov	bp,	GetSVGAVBEInfoErrMessage
	int	10h

	jmp	$

.KO:

	mov	ax,	1301h
	mov	bx,	000Fh
	mov	dx,	0A00h		;row 10
	mov	cx,	29
	push	ax
	mov	ax,	ds
	mov	es,	ax
	pop	ax
	mov	bp,	GetSVGAVBEInfoOKMessage
	int	10h

;=======	Get SVGA Mode Info

	mov	ax,	1301h
	mov	bx,	000Fh
	mov	dx,	0C00h		;row 12
	mov	cx,	24
	push	ax
	mov	ax,	ds
	mov	es,	ax
	pop	ax
	mov	bp,	StartGetSVGAModeInfoMessage
	int	10h


	mov	ax,	0x00
	mov	es,	ax
	mov	si,	0x800e

	mov	esi,	dword	[es:si]
	mov	edi,	0x8200

Label_SVGA_Mode_Info_Get:

	mov	cx,	word	[es:esi]

;=======	display SVGA mode information

	push	ax
	
	mov	ax,	00h
	mov	al,	ch
	call	Label_DispAL

	mov	ax,	00h
	mov	al,	cl	
	call	Label_DispAL
	
	pop	ax

;=======
	
	cmp	cx,	0FFFFh
	jz	Label_SVGA_Mode_Info_Finish

	mov	ax,	4F01h
	int	10h

	cmp	ax,	004Fh

	jnz	Label_SVGA_Mode_Info_FAIL	

	add	esi,	2
	add	edi,	0x100

	jmp	Label_SVGA_Mode_Info_Get
		
Label_SVGA_Mode_Info_FAIL:

	mov	ax,	1301h
	mov	bx,	008Ch
	mov	dx,	0D00h		;row 13
	mov	cx,	24
	push	ax
	mov	ax,	ds
	mov	es,	ax
	pop	ax
	mov	bp,	GetSVGAModeInfoErrMessage
	int	10h

Label_SET_SVGA_Mode_VESA_VBE_FAIL:

	jmp	$

Label_SVGA_Mode_Info_Finish:

	mov	ax,	1301h
	mov	bx,	000Fh
	mov	dx,	0E00h		;row 14
	mov	cx,	30
	push	ax
	mov	ax,	ds
	mov	es,	ax
	pop	ax
	mov	bp,	GetSVGAModeInfoOKMessage
	int	10h

; 只交接实际选定的模式，而不是让内核猜测分辨率/显存地址。
    xor ax, ax
    mov es, ax
    mov di, 0x8200
    mov cx, 0x180
    mov ax, 0x4f01
    int 0x10
    cmp ax, 0x004f
    jne Label_SET_SVGA_Mode_VESA_VBE_FAIL

;=======	set the SVGA mode(VESA VBE)

	mov	ax,	4F02h
	mov	bx,	4180h	;========================mode : 0x180 or 0x143
	int 	10h

	cmp	ax,	004Fh
	jnz	Label_SET_SVGA_Mode_VESA_VBE_FAIL

;=======	init IDT GDT goto protect mode 

	cli			;======close interrupt

	db	0x66
	lgdt	[GdtPtr]

;	db	0x66
;	lidt	[IDT_POINTER]

	mov	eax,	cr0
	or	eax,	1
	mov	cr0,	eax	

	jmp	dword SelectorCode32:GO_TO_TMP_Protect

[SECTION .s32]
[BITS 32]

GO_TO_TMP_Protect:

; 已由前面的远跳转加载 CODE32。CR0.PE 负责启用保护模式，远跳转负责加载
; CS 隐藏缓存中的 D 位；只有把两者结合起来，下面的 BITS 32 编码才匹配 CPU。
; 此阶段 PG=0、段 base=0，线性地址直接对应物理地址；之后不再调用实模式 BIOS。

	mov	ax,	0x10
	mov	ds,	ax
	mov	es,	ax
	mov	fs,	ax
	mov	ss,	ax
	mov	esp,	7E00h

	call	support_long_mode
	test	eax,	eax

	jz	no_support

;=======	init temporary page table 0x90000
    ; 页表项为 64 位，不能只写低 32 位并假定其余项已由 BIOS 清零。
    ; ES 现在是平坦 32 位数据段，清零 PML4/PDPT/PDE 共 3 个 4 KiB 页。
    cld
    xor eax, eax
    mov edi, 0x90000
    mov ecx, 3 * 4096 / 4
    rep stosd

	mov	dword	[0x90000],	0x91007
	mov	dword	[0x90800],	0x91007		

	mov	dword	[0x91000],	0x92007

	mov	dword	[0x92000],	0x000083

	mov	dword	[0x92008],	0x200083

	mov	dword	[0x92010],	0x400083

	mov	dword	[0x92018],	0x600083

	mov	dword	[0x92020],	0x800083

	mov	dword	[0x92028],	0xa00083

;=======	load GDTR

	db	0x66
	lgdt	[GdtPtr64]
	mov	ax,	0x10
	mov	ds,	ax
	mov	es,	ax
	mov	fs,	ax
	mov	gs,	ax
	mov	ss,	ax

	mov	esp,	7E00h

; IA-32e 的前置条件：先在 PG=0 的保护模式设置 CR4.PAE（bit5），再装 CR3。
; PAE 表示使用扩展页表机制，不表示直接进入 64 位模式；此处根是四级 PML4。

	mov	eax,	cr4
	bts	eax,	5
	mov	cr4,	eax

; CR3 必须是页表的物理地址。过渡表同时提供低地址与高半区别名，
; 保证开启分页后的当前指令、栈和将要跳入的内核代码仍可访问。

	mov	eax,	0x90000
	mov	cr3,	eax

; EFER.LME（bit8）是“允许进入 IA-32e”的请求；此时 PG=0，尚未激活。
; EFER.LMA（bit10）由 CPU 在条件满足时置位，不能把它当普通软件开关写入。

	mov	ecx,	0C0000080h		;IA32_EFER
	rdmsr

	bts	eax,	8
	wrmsr

; 最后置 CR0.PG（bit31）；PE/PAE/LME 都已满足，CPU 激活 IA-32e。
; 当前 CS 仍是 L=0 的 CODE32，因此先处于兼容子模式；紧随的远跳转加载
; CODE64（L=1、D=0），才开始按 64 位规则执行位于物理 1 MiB 的 head.S。

	mov	eax,	cr0
	bts	eax,	0
	bts	eax,	31
	mov	cr0,	eax

	jmp	SelectorCode64:OffsetOfKernelFile

;=======	test support long mode or not

support_long_mode:

	mov	eax,	0x80000000
	cpuid
	cmp	eax,	0x80000001
	setnb	al	
	jb	support_long_mode_done
	mov	eax,	0x80000001
	cpuid
	bt	edx,	29
	setc	al
support_long_mode_done:
	
	movzx	eax,	al
	ret

;=======	no support

no_support:
	jmp	$

;=======	read one sector from floppy

[SECTION .s16lib]
[BITS 16]

_ReadOneSectorFunc:
	
	push	bp
	mov	bp,	sp
	sub	esp,	2
	mov	byte	[bp - 2],	cl
	push	bx
	mov	bl,	[SectorsPerTrack]
	div	bl
	inc	ah
	mov	cl,	ah
	mov	dh,	al
	shr	al,	1
	mov	ch,	al
	and	dh,	1
	pop	bx
	mov	dl,	[DriveNumber]
_Go_On_Reading:
	mov	ah,	2
	mov	al,	byte	[bp - 2]
	int	13h
	jc	_Go_On_Reading
	add	esp,	2
	pop	bp
	ret

;=======	get FAT Entry

_GetFATEntryFunc:

	push	es
	push	bx
	push	ax
	mov	ax,	00
	mov	es,	ax
	pop	ax
	mov	byte	[Odd],	0
	mov	bx,	3
	mul	bx
	mov	bx,	2
	div	bx
	cmp	dx,	0
	jz	_Even
	mov	byte	[Odd],	1

_Even:
	xor	dx,	dx
	mov	bx,	[BytesPerSector]
	div	bx
	push	dx
	mov	bx,	8000h
	add	ax,	SectorNumOfFAT1Start
	mov	cl,	2
	call	_ReadOneSectorFunc
	
	pop	dx
	add	bx,	dx
	mov	ax,	[es:bx]
	cmp	byte	[Odd],	1
	jnz	_Even_2
	shr	ax,	4

_Even_2:
	and	ax,	0FFFh
	pop	bx
	pop	es
	ret

;=======	display num in al

Label_DispAL:

	push	ecx
	push	edx
	push	edi
	
	mov	edi,	[DisplayPosition]
	mov	ah,	0Fh
	mov	dl,	al
	shr	al,	4
	mov	ecx,	2
.begin:

	and	al,	0Fh
	cmp	al,	9
	ja	.1
	add	al,	'0'
	jmp	.2
.1:

	sub	al,	0Ah
	add	al,	'A'
.2:

	mov	[gs:edi],	ax
	add	edi,	2
	
	mov	al,	dl
	loop	.begin

	mov	[DisplayPosition],	edi

	pop	edi
	pop	edx
	pop	ecx
	
	ret


;=======	temp IDT

IDT:
	times	0x50	dq	0
IDT_END:

IDT_POINTER:
		dw	IDT_END - IDT - 1
		dd	IDT

;=======	temp variable

RootDirSizeForLoop	dw	RootDirSectors
SectorNo		dw	0
Odd			db	0
OffsetOfKernelFileCount	dd	OffsetOfKernelFile

DisplayPosition		dd	0

;=======	display messages

StartLoaderMessage: db "Start Miko OS Loader" 
NoKernelMessage: db "ERROR:No Kernel Found"
KernelFileName:		db	"KERNEL  BIN",0
StartGetMemStructMessage:	db	"Start Get Memory Struct."
GetMemStructErrMessage:	db	"Get Memory Struct ERROR"
GetMemStructOKMessage:	db	"Get Memory Struct SUCCESSFUL!"

StartGetSVGAVBEInfoMessage:	db	"Start Get SVGA VBE Info"
GetSVGAVBEInfoErrMessage:	db	"Get SVGA VBE Info ERROR"
GetSVGAVBEInfoOKMessage:	db	"Get SVGA VBE Info SUCCESSFUL!"

StartGetSVGAModeInfoMessage:	db	"Start Get SVGA Mode Info"
GetSVGAModeInfoErrMessage:	db	"Get SVGA Mode Info ERROR"
GetSVGAModeInfoOKMessage:	db	"Get SVGA Mode Info SUCCESSFUL!"
