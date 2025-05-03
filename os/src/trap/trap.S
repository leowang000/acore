.altmacro

.macro SAVE_GPR id
    sd x\id, \id * 8(sp)
.endm

.macro LOAD_GPR id
    ld x\id, \id * 8(sp)
.endm

    .section .text.trampoline
    .globl __alltraps
    .align 2
__alltraps:
    # now in user address space
    # sp->user_sp, sscratch->TRAP_CONTEXT
    csrrw sp, sscratch, sp
    # now sp->TRAP_CONTEXT, sscratch->user_sp
    # save GPRs to the TrapContext page, except x0, sp(x2) and tp(x4)
    SAVE_GPR 1
    SAVE_GPR 3
    .set n, 5
    .rept 27
        SAVE_GPR %n
        .set n, n + 1
    .endr
    # save CSR sstatus and sepc to the TrapContext page
    csrr t0, sstatus
    sd t0, 32 * 8(sp)
    csrr t0, sepc
    sd t0, 33 * 8(sp)
    # save the sp before entering the trap (i.e. the user_sp)
    csrr t0, sscratch
    sd t0, 2 * 8(sp)
    # load kernel_satp into t0
    ld t0, 34 * 8(sp)
    # load trap_handler into t1
    ld t1, 36 * 8(sp)
    # switch to kernel stack
    ld sp, 35 * 8(sp)
    # switch to kernel address space
    csrw satp, t0
    sfence.vma
    # jump to trap_handler
    jr t1

    .globl __restore
__restore:
    # a0: the va of the TrapContext page (i.e. TRAP_CONTEXT)
    # a1: user address space satp
    # switch to user address space
    csrw satp, a1
    sfence.vma
    csrw sscratch, a0
    mv sp, a0
    # now sp->TRAP_CONTEXT, sscratch->TRAP_CONTEXT
    # restore GPRs, except x0, sp(x2) and tp(x4)
    LOAD_GPR 1
    LOAD_GPR 3
    .set n, 5
    .rept 27
        LOAD_GPR %n
        .set n, n + 1
    .endr
    # restore CSR sstatus and sepc
    ld t0, 32 * 8(sp)
    csrw sstatus, t0
    ld t0, 33 * 8(sp)
    csrw sepc, t0
    # back to user stack
    ld sp, 2 * 8(sp)
    sret