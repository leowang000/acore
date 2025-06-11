.altmacro

.macro SAVE_SREG id
    sd s\id, (\id + 2) * 8(a0)
.endm

.macro LOAD_SREG id
    ld s\id, (\id + 2) * 8(a1)
.endm

    .section .text
    .globl __switch
__switch:
    sd sp, 0(a0)
    sd ra, 8(a0)
    .set n, 0
    .rept 12
        SAVE_SREG %n
        .set n, n + 1
    .endr
    ld sp, 0(a1)
    ld ra, 8(a1)
    .set n, 0
    .rept 12
        LOAD_SREG %n
        .set n, n + 1
    .endr
    ret
