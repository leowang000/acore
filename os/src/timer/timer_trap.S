    .section .text
    .globl __timer_trap
    .align 2
__timer_trap:
    csrrw sp, mscratch, sp
    # Store t0, t1, t2.
    sd t0, 0 * 8(sp)
    sd t1, 1 * 8(sp)
    sd t2, 2 * 8(sp)
    # Load the address of MTIMERCMP into t0.
    ld t0, 3 * 8(sp)
    # Load the time interval into t1.
    ld t1, 4 * 8(sp)
    # Load the current time to t2.
    ld t2, 0(t0)
    # Get the next trigger time.
    add t2, t2, t1
    # Store the next trigger time to MTIMECMP
    sd t2, 0(t0)
    # Set SSIP bit (value 2) in SIP register to trigger supervisor-mode software interrupt.
    li t0, 2
    csrw sip, t0
    # Restore t0, t1, t2.
    ld t0, 0 * 8(sp)
    ld t1, 1 * 8(sp)
    ld t2, 2 * 8(sp)
    csrrw sp, mscratch, sp
    mret