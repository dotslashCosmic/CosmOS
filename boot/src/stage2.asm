; CosmosBootloader 64-bit Stage

[BITS 16]
[ORG 0x8000]

; Constants for kernel loading
KERNEL_TEMP_SEGMENT     equ 0x1000      ; Temporary load at 0x10000
KERNEL_FINAL_ADDRESS    equ 0x200000    ; Final kernel location, 2MB
KERNEL_SECTORS          equ 127         ; Number of sectors to load
KERNEL_START_SECTOR     equ 66          ; Starting sector, MBR + 64x sectors
STACK_ADDRESS           equ 0x90000     ; Stack location
KERNEL_SIGNATURE        equ 0xC05305    ; "CosmOS" kernel signature
MAX_RETRIES             equ 3           ; Maximum retry attempts

; Memory detection constants
MEMORY_MAP_LOCATION     equ 0x9000      ; Memory map storage location
E820_SIGNATURE          equ 0x534D4150  ; "SMAP" signature for E820
MAX_MEMORY_ENTRIES      equ 32          ; Maximum memory map entries

; Memory map entry structure (24 bytes each)
MEMMAP_ENTRY_SIZE       equ 24

; Memory types (E820 standard)
MEMTYPE_USABLE          equ 1           ; Available RAM
MEMTYPE_RESERVED        equ 2           ; Reserved by system
MEMTYPE_ACPI_RECLAIM    equ 3           ; ACPI reclaimable
MEMTYPE_ACPI_NVS        equ 4           ; ACPI NVS
MEMTYPE_BAD             equ 5           ; Bad memory

stage2_start:
    ; Print Stage 2 message
    mov si, msg_stage2
    call print_string
    
    ; Detect memory using E820
    mov si, msg_detect_memory
    call print_string
    call detect_memory_e820
    
    ; Load kernel from disk
    mov si, msg_load_kernel
    call print_string

    ; Try LBA read first
    mov cx, MAX_RETRIES
.retry_lba:
    push cx                 ; Save retry counter
    call load_kernel_lba
    mov bx, ax              ; Save error info
    pop cx                  ; Restore retry counter
    jnc kernel_load_success ; Success if CF clear
    
    ; Check error code for fallback conditions
    mov ah, bl             ; Restore error
    cmp ah, 0x01           ; Invalid command
    je .try_chs            ; Fall back to CHS
    cmp ah, 0x0C           ; Unsupported track
    je .try_chs            ; Fall back to CHS
    
    ; Reset disk and retry
    push cx
    call reset_disk
    pop cx
    dec cx
    jnz .retry_lba
    
.try_chs:
    ; Fall back to CHS read
    mov si, msg_fallback_chs
    call print_string
    call load_kernel_chs
    jc kernel_load_error
    
kernel_load_success:
    ; Verify kernel signature before proceeding
    call verify_kernel_signature
    jc kernel_signature_error
    
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

; E820 Memory Detection Function
detect_memory_e820:
    push ax
    push bx
    push cx
    push dx
    push di
    push es
    
    ; Set up ES:DI to point to memory map storage
    mov ax, 0
    mov es, ax
    mov di, MEMORY_MAP_LOCATION
    
    ; Initialize entry counter
    mov word [memory_entry_count], 0
    
    ; Store entry count location at start of memory map
    mov word [di], 0        ; Entry count
    add di, 4               ; Move past entry count
    
    xor ebx, ebx            ; EBX = 0 for first call
    mov edx, E820_SIGNATURE ; EDX = "SMAP"
    
.e820_loop:
    mov eax, 0xE820         ; E820 function
    mov ecx, 24             ; Buffer size
    int 0x15                ; Call BIOS
    
    ; Check for errors
    jc .e820_failed         ; Carry flag set = error
    cmp eax, E820_SIGNATURE ; EAX should contain "SMAP"
    jne .e820_failed
    
    ; Check if entry is valid (length > 0)
    cmp dword [es:di+8], 0  ; Check length low dword
    je .skip_entry
    cmp dword [es:di+12], 0 ; Check length high dword
    jne .valid_entry
    cmp dword [es:di+8], 0  ; If high is 0, low must be > 0
    je .skip_entry
    
.valid_entry:
    ; Validate memory type and entry consistency
    call validate_memory_entry
    jc .skip_entry          ; Skip invalid entries
    
.entry_ok:
    ; Increment entry count
    inc word [memory_entry_count]
    
    ; Move to next entry location
    add di, 24              ; Each entry is 24 bytes
    
    ; Check if we've reached maximum entries
    cmp word [memory_entry_count], MAX_MEMORY_ENTRIES
    jae .e820_done
    
.skip_entry:
    ; Check if this was the last entry
    test ebx, ebx
    jz .e820_done
    
    ; Continue with next entry
    jmp .e820_loop
    
.e820_failed:
    ; E820 failed, try fallback
    mov si, msg_e820_failed
    call print_string
    call detect_memory_fallback
    jmp .detection_done
    
.e820_done:
    ; Update entry count at start of memory map
    mov di, MEMORY_MAP_LOCATION
    mov ax, word [memory_entry_count]
    mov word [di], ax
    
    ; Validate and count all entries
    call count_memory_entries
    
    ; Display detection results
    mov si, msg_memory_detected
    call print_string
    mov ax, word [memory_entry_count]
    call print_hex_word
    mov si, msg_entries
    call print_string
    
    ; Display first few entries for debugging
    call display_memory_map
    
.detection_done:
    pop es
    pop di
    pop dx
    pop cx
    pop bx
    pop ax
    ret

; Fallback memory detection using INT 15h AX=E801h
detect_memory_fallback:
    push ax
    push bx
    push cx
    push dx
    push di
    push es
    
    ; Try INT 15h, AX=E801h first
    mov ax, 0xE801
    int 0x15
    jc .try_88h
    
    ; E801h successful - AX/CX = memory 1MB-16MB in KB, BX/DX = memory >16MB in 64KB blocks, create synthetic memory map entries
    mov ax, 0
    mov es, ax
    mov di, MEMORY_MAP_LOCATION
    
    ; Entry count = 2, low and high memory
    mov word [di], 2
    add di, 4
    
    ; First entry: 0x0 - 0x9FC00
    mov dword [di], 0x0         ; Base address low
    mov dword [di+4], 0x0       ; Base address high
    mov dword [di+8], 0x9FC00   ; Length low (~640KB)
    mov dword [di+12], 0x0      ; Length high
    mov dword [di+16], 1        ; Type: usable
    mov dword [di+20], 0        ; Attributes
    add di, 24
    
    ; Second entry: 0x100000 - end of memory
    mov dword [di], 0x100000    ; Base address low
    mov dword [di+4], 0x0       ; Base address high
    
    ; Calculate total memory from E801h
    ; AX = KB between 1MB-16MB, BX = 64KB blocks above 16MB
    mov eax, 0xE801
    int 0x15
    
    ; Convert to bytes: AX * 1024 + BX * 65536
    movzx eax, ax
    mov ecx, 1024
    mul ecx                     ; EAX = low memory in bytes
    movzx ebx, bx
    mov ecx, 65536
    push eax
    mov eax, ebx
    mul ecx                     ; EAX = high memory in bytes
    pop ebx
    add eax, ebx                ; Total extended memory
    
    mov dword [di+8], eax       ; Length low
    mov dword [di+12], 0        ; Length high
    mov dword [di+16], 1        ; Type: usable
    mov dword [di+20], 0        ; Attributes
    
    mov word [memory_entry_count], 2
    jmp .fallback_done
    
.try_88h:
    ; Try INT 15h, AH=88h as last resort
    mov ah, 0x88
    int 0x15
    jc .fallback_failed
    
    ; AX = KB of extended memory
    test ax, ax
    jz .fallback_failed
    
    ; Create minimal memory map
    mov bx, 0
    mov es, bx
    mov di, MEMORY_MAP_LOCATION
    
    ; Entry count = 1
    mov word [di], 1
    add di, 4
    
    ; Single entry: 0x100000 - (1MB + AX*1024)
    mov dword [di], 0x100000    ; Base address low, 1MB
    mov dword [di+4], 0x0       ; Base address high
    movzx eax, ax
    mov ecx, 1024
    mul ecx                     ; Convert KB to bytes
    mov dword [di+8], eax       ; Length low
    mov dword [di+12], 0        ; Length high
    mov dword [di+16], 1        ; Type: usable
    mov dword [di+20], 0        ; Attributes
    
    mov word [memory_entry_count], 1
    jmp .fallback_done
    
.fallback_failed:
    ; All methods failed - create minimal default
    mov bx, 0
    mov es, bx
    mov di, MEMORY_MAP_LOCATION
    
    mov word [di], 1            ; One entry
    add di, 4
    
    ; Assume 16MB minimum
    mov dword [di], 0x100000    ; Base: 1MB
    mov dword [di+4], 0x0
    mov dword [di+8], 0xF00000  ; Length: 15MB
    mov dword [di+12], 0x0
    mov dword [di+16], 1        ; Type: usable
    mov dword [di+20], 0
    
    mov word [memory_entry_count], 1
    
.fallback_done:
    pop es
    pop di
    pop dx
    pop cx
    pop bx
    pop ax
    ret

; Validate memory map entry at ES:DI
validate_memory_entry:
    push eax
    push ebx
    
    ; Check memory type
    mov eax, dword [es:di+16]
    cmp eax, MEMTYPE_USABLE
    jb .invalid             ; Type < 1 is invalid
    cmp eax, MEMTYPE_BAD
    jbe .valid              ; Types 1-5 are standard
    cmp eax, 12
    ja .invalid             ; Extended types up to 12
    
    ; Check base address alignment
    mov eax, dword [es:di]      ; Base address low
    mov ebx, dword [es:di+4]    ; Base address high
    
    ; Reject obviously invalid addresses (> 1TB)
    cmp ebx, 0x10000
    jae .invalid
    
    ; Check length is reasonable, non-zero, too large
    mov eax, dword [es:di+8]    ; Length low
    mov ebx, dword [es:di+12]   ; Length high
    
    ; Length must be non-zero
    test eax, eax
    jnz .check_length_high
    test ebx, ebx
    jz .invalid
    
.check_length_high:
    ; Length should not exceed reasonable limits, 1TB
    cmp ebx, 0x10000
    jae .invalid
    
.valid:
    clc                     ; Clear carry, valid
    jmp .done
    
.invalid:
    stc                     ; Set carry, invalid
    
.done:
    pop ebx
    pop eax
    ret

; Count and validate all memory entries
count_memory_entries:
    push ax
    push bx
    push cx
    push di
    push es
    
    mov ax, 0
    mov es, ax
    mov di, MEMORY_MAP_LOCATION
    
    ; Get entry count
    mov cx, word [di]
    add di, 4               ; Skip to first entry
    
    mov bx, 0               ; Valid entry counter
    
.count_loop:
    test cx, cx
    jz .count_done
    
    ; Validate this entry
    call validate_memory_entry
    jc .skip_count
    
    inc bx                  ; Count valid entry
    
.skip_count:
    add di, MEMMAP_ENTRY_SIZE
    dec cx
    jmp .count_loop
    
.count_done:
    ; Update entry count with valid entries only
    mov di, MEMORY_MAP_LOCATION
    mov word [di], bx
    mov word [memory_entry_count], bx
    
    pop es
    pop di
    pop cx
    pop bx
    pop ax
    ret

; Display memory map for debugging
display_memory_map:
    push ax
    push bx
    push cx
    push dx
    push si
    push di
    push es
    
    mov ax, 0
    mov es, ax
    mov di, MEMORY_MAP_LOCATION + 4  ; Skip entry count
    mov cx, word [memory_entry_count]
    test cx, cx
    jz .display_done
    
    ; Limit display to first 4 entries to avoid screen overflow
    cmp cx, 4
    jbe .display_loop
    mov cx, 4
    
.display_loop:
    push cx
    
    ; Display base address
    mov si, msg_base_addr
    call print_string
    mov eax, dword [es:di+4]    ; High dword
    call print_hex_dword
    mov eax, dword [es:di]      ; Low dword
    call print_hex_dword
    
    ; Display length
    mov si, msg_length
    call print_string
    mov eax, dword [es:di+12]   ; High dword
    call print_hex_dword
    mov eax, dword [es:di+8]    ; Low dword
    call print_hex_dword
    
    ; Display type with description
    mov si, msg_type
    call print_string
    mov eax, dword [es:di+16]
    call print_hex_dword
    
    ; Add type description
    mov si, msg_space
    call print_string
    cmp eax, MEMTYPE_USABLE
    je .type_usable
    cmp eax, MEMTYPE_RESERVED
    je .type_reserved
    cmp eax, MEMTYPE_ACPI_RECLAIM
    je .type_acpi_reclaim
    cmp eax, MEMTYPE_ACPI_NVS
    je .type_acpi_nvs
    cmp eax, MEMTYPE_BAD
    je .type_bad
    
    mov si, msg_type_unknown
    jmp .type_done
    
.type_usable:
    mov si, msg_type_usable_desc
    jmp .type_done
.type_reserved:
    mov si, msg_type_reserved_desc
    jmp .type_done
.type_acpi_reclaim:
    mov si, msg_type_acpi_reclaim_desc
    jmp .type_done
.type_acpi_nvs:
    mov si, msg_type_acpi_nvs_desc
    jmp .type_done
.type_bad:
    mov si, msg_type_bad_desc
    
.type_done:
    call print_string
    mov si, msg_newline
    call print_string
    
    add di, 24                  ; Next entry
    pop cx
    dec cx
    jnz .display_loop
    
.display_done:
    pop es
    pop di
    pop si
    pop dx
    pop cx
    pop bx
    pop ax
    ret

; Load kernel using LBA, INT 13h, AH=42h
load_kernel_lba:
    ; Build proper 16-byte DAP in memory
    mov ax, KERNEL_TEMP_SEGMENT
    mov es, ax
    xor bx, bx              ; ES:BX = load address
    
    ; Build DAP at a fixed memory location
    mov di, dap_buffer
    mov byte [di], 16       ; Size of DAP
    mov byte [di+1], 0      ; Reserved
    mov word [di+2], KERNEL_SECTORS  ; Number of sectors
    mov word [di+4], 0      ; Offset 0
    mov word [di+6], KERNEL_TEMP_SEGMENT   ; Segment
    mov dword [di+8], KERNEL_START_SECTOR  ; LBA low 32 bits
    mov dword [di+12], 0    ; LBA high 32 bits
    
    mov si, di              ; DS:SI points to DAP
    mov ah, 0x42            ; Extended read
    mov dl, 0x80            ; First hard disk
    int 0x13
    
    ; Check return status carefully and store error code
    jc .error               ; CF set indicates error
    test ah, ah             ; AH should be 0 on success
    jnz .error
    clc                     ; Ensure CF is clear on success
    ret
    
.error:
    ; Store error code for debugging
    mov byte [disk_error_code], ah
    stc                     ; Ensure CF is set
    ret

; Load kernel using CHS, INT 13h, AH=02h
load_kernel_chs:
    ; Convert LBA to CHS
    ; LBA = (C × HPC + H) × SPT + (S - 1)
    ; Standard: 16 heads, 63 sectors per track
    mov ax, KERNEL_TEMP_SEGMENT
    mov es, ax
    xor bx, bx              ; ES:BX = load address
    
    ; Convert LBA 66 to CHS
    mov ax, KERNEL_START_SECTOR  ; LBA
    mov bl, 63              ; Sectors per track
    div bl                  ; AX / 63 = quotient in AL, remainder in AH
    mov cl, ah              ; Sector = remainder
    inc cl                  ; Sectors are 1-based
    
    xor ah, ah              ; Clear remainder
    mov bl, 16              ; Heads per cylinder  
    div bl                  ; AL / 16 = cylinder, AH = head
    mov ch, al              ; Cylinder
    mov dh, ah              ; Head
    
    ; Limit sectors for CHS, max 63 sectors per call
    mov al, KERNEL_SECTORS
    cmp al, 63
    jbe .read_sectors
    mov al, 63              ; Limit to 63 sectors
    
.read_sectors:
    mov ah, 0x02            ; Read sectors
    mov dl, 0x80            ; Drive
    int 0x13
    
    jc .error
    test ah, ah
    jnz .error
    clc
    ret
    
.error:
    mov byte [disk_error_code], ah
    stc
    ret

; Reset disk drive
reset_disk:
    mov ah, 0x00            ; Reset disk
    mov dl, 0x80            ; First hard disk
    int 0x13
    ret

; Verify kernel has CosmOS signature, 64-bit, 0xF(major)F(minor)F(patch)FC05305
verify_kernel_signature:
    push ax
    push bx
    push cx
    push dx
    push si
    push di
    push es
    
    mov ax, KERNEL_TEMP_SEGMENT
    mov es, ax
    
    ; Constants for signature search
    mov ecx, 65536          ; Search first 64KB
    mov ebx, KERNEL_SECTORS * 512  ; Actual kernel size
    cmp ecx, ebx
    jbe .search_size_ok
    mov ecx, ebx            ; Use kernel size if smaller than 64KB
    
.search_size_ok:
    xor si, si              ; Offset = 0
    
.search_loop:
    ; Check if we have 8 bytes left to read
    mov eax, ecx
    sub eax, esi
    cmp eax, 8
    jb .not_found           ; Less than 8 bytes left
    
    ; Read 64-bit value at ES:SI
    ; Read lower 32 bits
    mov eax, dword [es:si]
    ; Read upper 32 bits
    mov edx, dword [es:si+4]
    
    ; Check if lower 28 bits match 0xFC05305
    ; Mask: 0x0FFFFFFF (28 bits)
    and eax, 0x0FFFFFFF
    cmp eax, 0x0FC05305
    je .found_signature
    
    ; Move to next 8-byte aligned position
    add si, 8
    jmp .search_loop
    
.found_signature:
    ; Valid CosmOS signature found
    clc                     ; Clear carry - valid
    jmp .done
    
.not_found:
    ; Signature not found
    stc                     ; Set carry - invalid
    
.done:
    pop es
    pop di
    pop si
    pop dx
    pop cx
    pop bx
    pop ax
    ret

kernel_load_error:
    mov si, msg_kernel_error
    call print_string
    
    ; Display error code
    mov si, msg_error_code
    call print_string
    mov al, byte [disk_error_code]
    call print_hex_byte
    mov si, msg_newline
    call print_string
    jmp hang

kernel_signature_error:
    mov si, msg_signature_error
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
msg_detect_memory   db 'Detecting system memory...', 13, 10, 0
msg_load_kernel     db 'Loading kernel from disk...', 13, 10, 0
msg_fallback_chs    db 'LBA failed, trying CHS...', 13, 10, 0
msg_kernel_loaded   db 'Kernel loaded successfully', 13, 10, 0
msg_temp_addr       db 'Temp location: 0x', 0
msg_final_addr      db ', Final: 0x', 0
msg_sectors_loaded  db ', Sectors: 0x', 0
msg_newline         db 13, 10, 0
msg_protected       db 'Protected mode active, setting up paging...', 13, 10, 0
msg_kernel_error    db 'Kernel load failed! Check disk configuration.', 13, 10, 0
msg_signature_error db 'Invalid kernel signature or checksum!', 13, 10, 0
msg_error_code      db 'BIOS Error Code: 0x', 0
msg_e820_failed     db 'E820 detection failed, using fallback...', 13, 10, 0
msg_memory_detected db 'Memory detected: 0x', 0
msg_entries         db ' entries', 13, 10, 0
msg_base_addr       db 'Base: 0x', 0
msg_length          db ' Len: 0x', 0
msg_type            db ' Type: 0x', 0
msg_space           db ' ', 0
msg_type_usable_desc        db '(Usable)', 0
msg_type_reserved_desc      db '(Reserved)', 0
msg_type_acpi_reclaim_desc  db '(ACPI)', 0
msg_type_acpi_nvs_desc      db '(ACPI NVS)', 0
msg_type_bad_desc           db '(Bad)', 0
msg_type_unknown            db '(Unknown)', 0

; DAP buffer for LBA reads, 16 bytes aligned
align 4
dap_buffer:
    times 16 db 0

; Error tracking
disk_error_code db 0

; Memory detection tracking
memory_entry_count dw 0

; GDT for protected mode, brevity fix
align 8
gdt_start:
    ; Null descriptor
    dq 0x0000000000000000
    
    ; Code segment, 32-bit, selector 0x08
    dq 0x00CF9A000000FFFF   ; Base=0, Limit=0xFFFFF, Present, Ring0, Code, 32-bit
    
    ; Data segment, 32-bit, selector 0x10 
    dq 0x00CF92000000FFFF   ; Base=0, Limit=0xFFFFF, Present, Ring0, Data, 32-bit
    
    ; Code segment, 64-bit, selector 0x18
    dq 0x00AF9A000000FFFF   ; Base=0, Limit=0xFFFFF, Present, Ring0, Code, 64-bit
    
    ; Data segment, 64-bit, selector 0x20
    dq 0x00AF92000000FFFF   ; Base=0, Limit=0xFFFFF, Present, Ring0, Data, 64-bit

gdt_end:

gdt_descriptor:
    dw gdt_end - gdt_start - 1  ; Size
    dd gdt_start                ; Offset

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
    
    ; Copy KERNEL_SECTORS * 512 bytes to final
    mov esi, KERNEL_TEMP_SEGMENT * 16  ; Source: temporary load location
    mov edi, KERNEL_FINAL_ADDRESS      ; Destination: final kernel location
    mov ecx, KERNEL_SECTORS * 512 / 4  ; Convert to dwords (127 * 512 / 4 = 16256)
    
    ; Perform the copy (signature already verified in 16-bit mode)
    cld                     ; Ensure forward direction
    rep movsd
    
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
    
    ; Set up page tables with proper flag composition, PML4 at 0x70000
    mov edi, 0x70000
    mov eax, 0x71000        ; PDPT base address
    or eax, 0x03            ; Add present + writable flags
    mov dword [edi], eax    ; Lower 32 bits
    mov dword [edi+4], 0    ; Upper 32 bits
    
    ; PDPT at 0x71000
    mov edi, 0x71000
    mov eax, 0x72000        ; PD base address
    or eax, 0x03            ; Add present + writable flags
    mov dword [edi], eax    ; Lower 32 bits
    mov dword [edi+4], 0    ; Upper 32 bits
    
    ; Calculate how much memory to map from E820
    call calculate_pages_to_map  ; Returns page count in ECX
    push ecx                     ; Save for later
    
    ; Calculate PD count (pages / 512, rounded up)
    mov eax, ecx
    add eax, 511
    shr eax, 9              ; Divide by 512
    test eax, eax
    jnz .pd_count_ok
    mov eax, 1              ; Minimum 1 PD
.pd_count_ok:
    push eax                ; Save PD count
    
    ; Set up PDPT entries for all PDs
    mov edi, 0x71000
    mov esi, 0x72000        ; First PD address
    mov ebx, eax            ; PD count
.setup_pdpt_loop:
    test ebx, ebx
    jz .pdpt_done
    mov eax, esi
    or eax, 0x03
    mov [edi], eax
    mov dword [edi+4], 0
    add edi, 8
    add esi, 0x1000
    dec ebx
    jmp .setup_pdpt_loop
.pdpt_done:
    
    ; Map all pages
    pop eax                 ; Restore PD count (not needed)
    pop ecx                 ; Restore page count
    mov edi, 0x72000
    mov edx, 0
.map_all_pages:
    test ecx, ecx
    jz .mapping_done
    mov eax, edx
    or eax, 0x83
    mov [edi], eax
    mov dword [edi+4], 0
    add edx, 0x200000
    add edi, 8
    dec ecx
    jmp .map_all_pages
.mapping_done:
    
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

; Calculate pages to map based on E820 memory map
; Returns: ECX = number of 2MB pages to map
calculate_pages_to_map:
    push eax
    push ebx
    push edx
    push esi
    
    ; Get entry count from memory map
    mov esi, MEMORY_MAP_LOCATION
    movzx ebx, word [esi]    ; Entry count
    add esi, 4               ; Skip to first entry
    
    xor edx, edx             ; EDX = highest usable RAM address found
    
.scan_loop:
    test ebx, ebx
    jz .scan_done
    
    ; Check memory type (offset 16, 4 bytes)
    mov eax, [esi + 16]      ; Memory type
    cmp eax, MEMTYPE_USABLE  ; Only consider usable RAM (type 1)
    jne .next_entry
    
    ; Read entry: base (8 bytes) + length (8 bytes)
    mov eax, [esi + 8]       ; Length low 32 bits
    test eax, eax
    jz .next_entry
    
    ; Calculate end address = base + length
    mov eax, [esi]           ; Base low 32 bits
    add eax, [esi + 8]       ; Add length low 32 bits
    jc .next_entry           ; Skip if overflow
    
    ; Ignore addresses above 4GB (hardware mapped regions)
    test eax, eax
    js .next_entry           ; Skip if bit 31 set (>2GB might be hardware)
    
    ; Check if this is higher than current max
    cmp eax, edx
    jbe .next_entry
    mov edx, eax             ; Update highest address
    
.next_entry:
    add esi, 24              ; Next entry (24 bytes)
    dec ebx
    jmp .scan_loop
    
.scan_done:
    ; EDX now has highest usable RAM address
    ; Round up to 2MB boundary and convert to page count
    add edx, 0x1FFFFF        ; Round up
    shr edx, 21              ; Divide by 2MB (2^21)
    
    ; Ensure minimum of 64 pages (128MB)
    cmp edx, 64
    jae .count_ok
    mov edx, 64
.count_ok:
    
    ; Cap at 2048 pages (4GB)
    cmp edx, 2048
    jbe .no_cap
    mov edx, 2048
.no_cap:
    
    mov ecx, edx             ; Return in ECX
    
    pop esi
    pop edx
    pop ebx
    pop eax
    ret

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
    mov rcx, 2000           ; 80*25 = 2000 characters
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
