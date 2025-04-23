.altmacro

.macro SAVE_GPR id
    sd x\id, \id * 8(sp)
.endm

.macro LOAD_GPR id
    ld x\id, \id * 8(sp)
.endm

    .section .text
    .globl __alltraps
    .align 2
__alltraps:
    # sp: user stack->kernel stack
    # sscratch: kernel stack->user stack
    csrrw sp, sscratch, sp
    # reserve space for trap context
    addi sp, sp, -34 * 8
    # save GPRs, except x0, sp(x2) and tp(x4)
    SAVE_GPR 1
    SAVE_GPR 3
    .set n, 5
    .rept 27
        SAVE_GPR %n
        .set n, n + 1
    .endr
    csrr t0, sstatus
    sd t0, 32 * 8(sp)
    csrr t0, sepc
    sd t0, 33 * 8(sp)
    # save the sp before entering the trap (i.e. the user stack sp)
    csrr t0, sscratch
    sd t0, 2 * 8(sp)
    mv a0, sp
    call trap_handler

    .globl __restore
__restore:
    # reuse __restore to start the first task
    mv sp, a0
    LOAD_GPR 1
    LOAD_GPR 3
    .set n, 5
    .rept 27
        LOAD_GPR %n
        .set n, n + 1
    .endr
    ld t0, 32 * 8(sp)
    csrw sstatus, t0
    ld t0, 33 * 8(sp)
    csrw sepc, t0
    ld t0, 2 * 8(sp)
    csrw sscratch, t0
    addi sp, sp, 34 * 8
    csrrw sp, sscratch, sp
    sret