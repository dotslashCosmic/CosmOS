; CosmosBootloader 64-bit Stage

[BITS 16]
[ORG 0x8000]

; Constants for kernel loading
KERNEL_TEMP_SEGMENT     equ 0x1000      ; Temporary load at 0x10000
KERNEL_FINAL_ADDRESS    equ 0x200000    ; Final kernel location (2MB)
KERNEL_SECTORS          equ 127         ; Number of sectors to load
KERNEL_START_SECTOR     equ 66          ; Starting sector (MBR + 64x Stage 2 sectors)
STACK_ADDRESS           equ 0x90000     ; Stack location

stage2_start:
    ; Print Stage 2 message
    mov si, msg_stage2
    call print_string
    
    ; Load kernel from disk
    mov si, msg_load_kernel
    call print_string

    ; Load using LBA, BIOS Extended Read (INT 13h, AH=42h)
    mov ax, KERNEL_TEMP_SEGMENT
    mov es, ax
    
    ; Set up Disk Address Packet
    push dword 0            ; Upper 32 bits of LBA (0 for sector 66)
    push dword 66           ; Lower 32 bits of LBA (sector 66)
    push es                 ; Segment
    push word 0             ; Offset
    push word KERNEL_SECTORS ; Number of sectors to read
    push word 16            ; Size of DAP
    
    mov si, sp              ; DS:SI points to DAP
    mov ah, 0x42            ; Extended read
    mov dl, 0x80            ; First hard disk
    int 0x13
    jc kernel_load_error_lba
    
    add sp, 16              ; Clean up stack
    jmp kernel_load_success

kernel_load_error_lba:
    add sp, 16              ; Clean up stack
    jmp kernel_load_error

kernel_load_success:
    
    mov si, msg_kernel_loaded
    call print_string
    
    ; Display kernel details
    mov si, msg_temp_addr
    call print_string
    mov ax, KERNEL_TEMP_SEGMENT
    call print_hex_word
    
    mov si, msg_final_addr
    call print_string
    mov eax, KERNEL_FINAL_ADDRESS
    call print_hex_dword
    
    mov si, msg_sectors_loaded
    call print_string
    mov al, KERNEL_SECTORS
    call print_hex_byte
    mov si, msg_newline
    call print_string
    
    ; Enter protected mode
    mov si, msg_protected
    call print_string
    
    cli                     ; Disable interrupts
    lgdt [gdt_descriptor]   ; Load GDT
    
    ; Enable protected mode
    mov eax, cr0
    or eax, 1
    mov cr0, eax
    
    ; Far jump to 32-bit code
    jmp 0x08:protected_mode

kernel_load_error:
    mov si, msg_kernel_error
    call print_string
    jmp hang

; 16-bit print function
print_string:
    push ax
    push bx
.loop:
    lodsb
    test al, al
    jz .done
    mov ah, 0x0E
    mov bh, 0
    mov bl, 0x0F            ; White text
    int 0x10
    jmp .loop
.done:
    pop bx
    pop ax
    ret

; Print hex word, AX register, 16-bit
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
    mov bl, 0x0B            ; Light cyan
    int 0x10
    pop ax
    dec cx
    jnz .loop
    pop cx
    pop ax
    ret

; Print hex byte, AL register, 16-bit
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
    mov bl, 0x0B            ; Light cyan
    int 0x10
    pop ax
    dec cx
    jnz .loop
    pop cx
    pop ax
    ret

; Print hex dword, EAX register, 16-bit
print_hex_dword:
    push eax
    push ecx
    mov ecx, 8              ; 8 hex digits
.loop:
    rol eax, 4              ; Rotate left 4 bits
    push eax
    and al, 0x0F            ; Mask lower 4 bits
    add al, '0'             ; Convert to ASCII
    cmp al, '9'
    jbe .digit
    add al, 7               ; A-F adjustment
.digit:
    mov ah, 0x0E
    mov bh, 0
    mov bl, 0x0B            ; Light cyan
    int 0x10
    pop eax
    dec ecx
    jnz .loop
    pop ecx
    pop eax
    ret



hang:
    hlt
    jmp hang

; Messages
msg_stage2          db 'Stage 2 initialized - Setting up 64-bit...', 13, 10, 0
msg_load_kernel     db 'Loading kernel from disk...', 13, 10, 0
msg_kernel_loaded   db 'Kernel loaded successfully', 13, 10, 0
msg_temp_addr       db 'Temp location: 0x', 0
msg_final_addr      db ', Final: 0x', 0
msg_sectors_loaded  db ', Sectors: 0x', 0
msg_newline         db 13, 10, 0
msg_protected       db 'Protected mode active, setting up paging...', 13, 10, 0
msg_kernel_error    db 'Kernel load failed! Check disk configuration.', 13, 10, 0

; GDT for protected mode
align 8
gdt_start:
    ; Null descriptor
    dq 0
    
    ; Code segment, 32-bit
    dw 0xFFFF               ; Limit low
    dw 0                    ; Base low
    db 0                    ; Base middle
    db 10011010b            ; Access: present, ring 0, code, executable, readable
    db 11001111b            ; Flags: 4KB granularity, 32-bit
    db 0                    ; Base high
    
    ; Data segment, 32-bit
    dw 0xFFFF
    dw 0
    db 0
    db 10010010b            ; Access: present, ring 0, data, writable
    db 11001111b
    db 0
    
    ; Code segment, 64-bit
    dw 0xFFFF
    dw 0
    db 0
    db 10011010b            ; Access: present, ring 0, code, executable, readable
    db 10101111b            ; Flags: 4KB granularity, 64-bit
    db 0
    
    ; Data segment, 64-bit
    dw 0xFFFF
    dw 0
    db 0
    db 10010010b
    db 10101111b
    db 0

gdt_end:

gdt_descriptor:
    dw gdt_end - gdt_start - 1  ; Size
    dd gdt_start                 ; Offset

; 32-bit protected mode
[BITS 32]
protected_mode:
    ; Set up segments
    mov ax, 0x10            ; Data segment selector
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    mov esp, STACK_ADDRESS  ; Set up stack at 576KB
    
    ; Copy kernel from temp to final location
    ; Copy KERNEL_SECTORS * 512 bytes
    mov esi, KERNEL_TEMP_SEGMENT * 16  ; Source: temporary load location
    mov edi, KERNEL_FINAL_ADDRESS      ; Destination: final kernel location
    mov ecx, 16256          ; Copy 65024 bytes, (127 sectors * 512) / 4 = 16256 dwords
    
    ; Verify source has data before copying
    mov eax, dword [esi]
    test eax, eax
    jz copy_error
    
    rep movsd
    jmp copy_done

copy_error:
    ; Kernel wasn't loaded from disk
    mov esi, msg_copy_error
    call print_string_32
    jmp hang_32

copy_done:
    
    ; Check for long mode
    mov eax, 0x80000000
    cpuid
    cmp eax, 0x80000001
    jb no_long_mode
    
    mov eax, 0x80000001
    cpuid
    test edx, 1 << 29       ; Check LM bit
    jz no_long_mode
    
    ; Set up paging for long mode
    ; Identity map first 2MB
    
    ; Clear page tables area, 0x70000-0x90000
    mov edi, 0x70000
    mov ecx, 0x8000         ; 32KB
    xor eax, eax
    rep stosd
    
    ; Set up page tables
    ; PML4 at 0x70000
    mov edi, 0x70000
    mov dword [edi], 0x71003    ; Point to PDPT, present + writable
    mov dword [edi+4], 0x00000000   ; Upper 32 bits
    
    ; PDPT at 0x71000
    mov edi, 0x71000
    mov dword [edi], 0x72003    ; Point to PD, present + writable
    mov dword [edi+4], 0x00000000   ; Upper 32 bits
    
    ; PD at 0x72000 - identity map 32MB with 2MB pages (16 entries)
    mov edi, 0x72000
    mov eax, 0x00000083     ; Base flags: present + writable + 2MB page
    mov ecx, 16             ; Map 16 entries
    mov edx, 0              ; Physical address counter
.map_loop:
    mov [edi], eax          ; Lower 32 bits
    mov dword [edi+4], 0    ; Upper 32 bits, always 0 < 4GB
    add eax, 0x200000       ; Next 2MB page
    add edi, 8              ; Next page directory entry
    dec ecx
    jnz .map_loop
    
    ; Load CR3 with PML4 address
    mov eax, 0x70000
    mov cr3, eax
    
    ; Enable PAE
    mov eax, cr4
    or eax, 1 << 5
    mov cr4, eax
    
    ; Enable long mode
    mov ecx, 0xC0000080     ; EFER MSR
    rdmsr
    or eax, 1 << 8          ; Set LM bit
    wrmsr
    
    ; Enable paging
    mov eax, cr0
    or eax, 1 << 31
    mov cr0, eax
    
    ; Jump to 64-bit code
    jmp 0x18:long_mode

no_long_mode:
    ; Print error and hang
    mov esi, msg_no_long_mode
    call print_string_32
    jmp hang_32

print_string_32:
    push eax
    push ebx
    mov ebx, 0xB8000        ; VGA text buffer
.loop:
    lodsb
    test al, al
    jz .done
    mov byte [ebx], al
    mov byte [ebx+1], 0x0F  ; White on black
    add ebx, 2
    jmp .loop
.done:
    pop ebx
    pop eax
    ret

hang_32:
    hlt
    jmp hang_32

msg_no_long_mode db 'CPU is not 64-bit!', 0
msg_copy_error db 'ERROR: Kernel not loaded from disk!', 0

; 64-bit long mode code
[BITS 64]
long_mode:
    ; Set up segments for 64-bit
    mov ax, 0x20            ; 64-bit data segment
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax
    
    ; Set up stack
    mov rsp, STACK_ADDRESS
    
    ; Clear screen
    mov rdi, 0xB8000
    mov rcx, 80 * 25        ; 80x25 screen
    mov ax, 0x0F20          ; White space
    rep stosw
    
    ; Print success message
    mov rsi, msg_long_mode
    mov rdi, 0xB8000
    call print_string_64
    
    ; Verify kernel is loaded by checking first bytes
    mov al, byte [KERNEL_FINAL_ADDRESS]
    mov byte [0xB8140], al      ; Display first byte
    mov byte [0xB8141], 0x0E    ; Yellow
    
    mov al, byte [KERNEL_FINAL_ADDRESS + 1]
    mov byte [0xB8142], al      ; Display second byte
    mov byte [0xB8143], 0x0E    ; Yellow
    
    ; Display kernel address we're jumping to
    mov rdi, 0xB8144
    mov rax, KERNEL_FINAL_ADDRESS
    call write_hex_64
    
    ; Display first few bytes of kernel
    mov rdi, 0xB8164        ; Next line
    mov al, byte [KERNEL_FINAL_ADDRESS]
    call write_hex_byte_64
    mov al, byte [KERNEL_FINAL_ADDRESS + 1]
    call write_hex_byte_64
    mov al, byte [KERNEL_FINAL_ADDRESS + 2]
    call write_hex_byte_64
    mov al, byte [KERNEL_FINAL_ADDRESS + 3]
    call write_hex_byte_64
    
    ; Also check what's at the temporary load location
    mov rdi, 0xB8190        ; Next line
    mov al, byte [KERNEL_TEMP_SEGMENT * 16]
    call write_hex_byte_64
    mov al, byte [KERNEL_TEMP_SEGMENT * 16 + 1]
    call write_hex_byte_64
    mov al, byte [KERNEL_TEMP_SEGMENT * 16 + 2]
    call write_hex_byte_64
    mov al, byte [KERNEL_TEMP_SEGMENT * 16 + 3]
    call write_hex_byte_64
    
    ; Since it's flat binary, jump directly to load address
    mov rdi, 0xB81C0       ; Next line
    
    ; Show first 8 bytes of kernel
    mov rax, qword [KERNEL_FINAL_ADDRESS]
    call write_hex_64
    
    ; Check if valid x86-64 code
    mov rax, qword [KERNEL_FINAL_ADDRESS]
    test rax, rax
    jz kernel_not_loaded
    
    ; Jump directly to kernel start
    mov rax, qword KERNEL_FINAL_ADDRESS
    jmp rax

kernel_not_loaded:
    ; Display error message
    mov rdi, 0xB81E0       ; Next line
    mov rsi, msg_kernel_not_found
    call print_string_64
    cli
    hlt
    
    ; Should never execute
    mov word [0xB8144], 0x0C45  ; Red 'E'
    hlt

write_hex_64:
    ; Write 64-bit value in RAX as hex to RDI
    push rax
    push rcx
    mov rcx, 16
.loop:
    rol rax, 4
    push rax
    and rax, 0xF
    add al, '0'
    cmp al, '9'
    jbe .digit
    add al, 7
.digit:
    mov [rdi], al
    mov byte [rdi+1], 0x0F
    add rdi, 2
    pop rax
    dec rcx
    jnz .loop
    pop rcx
    pop rax
    ret

write_hex_byte_64:
    ; Write byte in AL as hex to RDI
    push rax
    push rcx
    mov rcx, 2
.loop:
    rol al, 4
    push rax
    and al, 0xF
    add al, '0'
    cmp al, '9'
    jbe .digit
    add al, 7
.digit:
    mov [rdi], al
    mov byte [rdi+1], 0x0E
    add rdi, 2
    pop rax
    dec rcx
    jnz .loop
    pop rcx
    pop rax
    ret

print_string_64:
    push rax
.loop:
    lodsb
    test al, al
    jz .done
    mov byte [rdi], al
    mov byte [rdi+1], 0x0A  ; Light green
    add rdi, 2
    jmp .loop
.done:
    pop rax
    ret

msg_long_mode db 'CosmosBootloader jumping to kernel...', 0
msg_kernel_not_found db 'ERROR: Kernel not loaded or invalid!', 0

; Pad Stage 2
times 32768-($-$$) db 0     ; 32KB total (64 sectors)
