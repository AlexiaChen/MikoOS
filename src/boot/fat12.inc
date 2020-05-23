; FAT12 Spec: http://www.maverick-os.dk/FileSystemFormats/FAT12_FileSystem.html
; File Allocation Table (FAT) = FAT1 + FAT2
; FAT12 = BootSector + FAT1 + FAT2 + Root Dir Region + Data Region
RootDirSectors equ 14 ; Root directory hold sector numbers, (NumberOfRootEntries * 32 + BytesPerSector - 1) / BytesPerSector = 14
SectorNumOfRootDirStart equ 19 ; Sector Number of start sector of root directory, (ReservedSectors + SectorsPerFAT*NumberOfFATCopies) = 19
SectorNumOfFAT1Start equ 1 ; Sector Number of syart sector of FAT1
SectorBalance equ 17 ; 19 - 2 = 17

    ; Boot Sector definitions
    OEMName db 'MikoBoot'
    BytesPerSector dw 512
    SectorsPerCluster db 1
    ReservedSectors dw 1
    NumberOfFATCopies db 2
    NumberOfRootEntries dw 224
    SmallNumberOfSectors dw 2880 ; Used when volume size is less than 32 Mb.
    MediaDesc db 0xf0
    SectorsPerFAT dw 9
    SectorsPerTrack dw 18
    NumberOfHeads dw 2
    HiddenSectors dd 0
    LargeNumberOfSectors dd 0 ; Used when volume size is greater than 32 Mb.
    DriveNumber db 0
    Reserved db 0 ; used by Windows NT to decide if it shall check disk integrity.
    BootSig db 0x29 ;  Indicates that the next three fields(VolumeSerialNumber, VolumeLabel, FileSystemType) are available.
    VolumeSerialNumber dd 0
    VolumeLabel db 'boot loader' ; must be 11 bytes string
    FileSystemType db 'FAT12   ' ; must be 8 bytes string