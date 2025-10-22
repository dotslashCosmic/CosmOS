; CosmosBootloader MBR Stage

[BITS 16]
[ORG 0x7C00]

; Constants for Stage 2 loading
DISK                equ 0x80        ; First hard disk
STAGE2_SEGMENT      equ 0x0800      ; 0x8000 physical
STAGE2_SECTORS      equ 64          ; Number of sectors to load
STAGE2_START        equ 2           ; Starting sector on disk

; Entry point
start:
    ; Disable interrupts during setup
    cli
    
    ; Set up segments
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7C00          ; Stack grows down from bootloader
    
    ; Enable A20 line required for accessing memory above 1MB
    call enable_a20
    
    ; Print boot message
    mov si, msg_boot
    call print_string
    
    ; Load Stage 2 bootloader from disk
    ; Stage 2 starts at sector 2 (sector 1 is MBR)
    mov si, msg_loading
    call print_string
    
    ; Load Stage 2 using constants
    mov ax, STAGE2_SEGMENT  ; Segment for Stage 2
    mov es, ax
    xor bx, bx              ; Offset 0
    mov ah, 0x02            ; BIOS read sectors
    mov al, STAGE2_SECTORS  ; Number of sectors to read
    mov ch, 0               ; Cylinder 0
    mov cl, STAGE2_START    ; Start from sector 2
    mov dh, 0               ; Head 0
    mov dl, DISK            ; First hard disk
    int 0x13
    jc disk_error
    
    ; Check if we loaded the right amount
    cmp al, STAGE2_SECTORS
    jne disk_error
    
    mov si, msg_loaded
    call print_string
    
    ; Display loading details
    mov si, msg_details
    call print_string
    
    ; Show segment address
    mov ax, STAGE2_SEGMENT
    call print_hex_word
    mov si, msg_colon
    call print_string
    
    ; Show sector count
    mov al, STAGE2_SECTORS
    call print_hex_byte
    mov si, msg_newline
    call print_string
    
    ; Jump to Stage 2
    jmp STAGE2_SEGMENT:0x0000

disk_error:
    mov si, msg_disk_error
    call print_string
    jmp hang

; Print string function, SI = pointer to null-terminated string
print_string:
    push ax
    push bx
.loop:
    lodsb                   ; Load byte from DS:SI into AL
    test al, al             ; Check for null terminator
    jz .done
    mov ah, 0x0E            ; BIOS teletype output
    mov bh, 0               ; Page 0
    mov bl, 0x07            ; Light gray
    int 0x10
    jmp .loop
.done:
    pop bx
    pop ax
    ret

; Print hex word, AX register
print_hex_word:
    push ax
    push cx
    mov cx, 4               ; 4 hex digits
.loop:
    rol ax, 4               ; Rotate left 4 bits
    push ax
    and al, 0x0F            ; Mask lower 4 bits
    add al, '0'             ; Convert to ASCII
    cmp al, '9'
    jbe .digit
    add al, 7               ; A-F adjustment
.digit:
    mov ah, 0x0E
    mov bh, 0
    mov bl, 0x0A            ; Light green
    int 0x10
    pop ax
    dec cx
    jnz .loop
    pop cx
    pop ax
    ret

; Print hex byte, AL register
print_hex_byte:
    push ax
    push cx
    mov cx, 2               ; 2 hex digits
.loop:
    rol al, 4               ; Rotate left 4 bits
    push ax
    and al, 0x0F            ; Mask lower 4 bits
    add al, '0'             ; Convert to ASCII
    cmp al, '9'
    jbe .digit
    add al, 7               ; A-F adjustment
.digit:
    mov ah, 0x0E
    mov bh, 0
    mov bl, 0x0A            ; Light green
    int 0x10
    pop ax
    dec cx
    jnz .loop
    pop cx
    pop ax
    ret

; Enable A20 line
enable_a20:
    push ax
    push dx

    ; Try A20
    in   al, 0x92
    test al, 2
    jnz  .done_fast          ; Already enabled
    or   al, 2
    and  al, 0xFE            ; Make sure bit 0 (reset) stays 0
    out  0x92, al
    in   al, 0x92
    test al, 2
    jnz  .done_fast          ; Success

    ; 8042 keyboard controller method fallback
    call .wait_input
    mov  al, 0xAD
    out  0x64, al            ; Disable keyboard

    call .wait_input
    mov  al, 0xD0
    out  0x64, al            ; Read output port
    call .wait_output
    in   al, 0x60
    or   al, 2               ; Set A20 enable bit
    call .wait_input
    mov  ah, al
    mov  al, 0xD1
    out  0x64, al
    call .wait_input
    mov  al, ah
    out  0x60, al

    call .wait_input
    mov  al, 0xAE
    out  0x64, al            ; Re-enable keyboard

.done_fast:
    pop  dx
    pop  ax
    ret

.wait_input:                 ; Wait for input buffer empty
    in   al, 0x64
    test al, 2
    jnz  .wait_input
    ret

.wait_output:                ; Wait for output buffer full
    in   al, 0x64
    test al, 1
    jz   .wait_output
    ret


hang:
    hlt
    jmp hang

; Messages
msg_boot        db 'CosmosBootloader', 13, 10, 0
msg_loading     db 'Loading Stage 2...', 13, 10, 0
msg_loaded      db 'Stage 2 loaded.', 13, 10, 0
msg_details     db 'Target: 0x', 0
msg_colon       db ', Sectors: 0x', 0
msg_newline     db 13, 10, 'Jumping to Stage 2...', 13, 10, 0
msg_disk_error  db 'Disk read error! Check boot device.', 13, 10, 0

; Pad to 510 bytes
times 510-($-$$) db 0
dw 0xAA55               ; Boot signature
