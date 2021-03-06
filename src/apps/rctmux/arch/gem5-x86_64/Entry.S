#
# Copyright (C) 2013, Nils Asmussen <nils@os.inf.tu-dresden.de>
# Economic rights: Technische Universitaet Dresden (Germany)
#
# This file is part of M3 (Microkernel for Minimalist Manycores).
#
# M3 is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License version 2 as
# published by the Free Software Foundation.
#
# M3 is distributed in the hope that it will be useful, but
# WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
# General Public License version 2 for more details.
#

#include <base/Asm.h>

.global rctmux_stack
.extern interrupt_handler
.extern init
.extern sleep

.text

// put that at the beginning so that we know the address
BEGIN_FUNC(_start)
    // we need nops here to ensure that we start executing at the first real instruction. because
    // the DTU simply sets the PC and it might be that the CPU finishes the last instruction
    // afterwards, which increases the PC. so, use more nops than the longest instruction
.rept 32
    nop
.endr

    // disable interrupts in case they are enabled
    cli

    // use a small stack
    mov     $rctmux_stack, %rsp
    mov     $0, %rbp

    call    init

    // return to idle code
    pushq   $(4 << 3) | 3   // ss
    pushq   $idle_stack     // rsp
    pushq   $0x200          // rflags: interrupts enabled
    pushq   $(3 << 3) | 3   // cs
    pushq   $idle           // rip
    iretq
END_FUNC(_start)

BEGIN_FUNC(idle)
1:  call    sleep
    jmp     1b
END_FUNC(idle)

BEGIN_FUNC(isr_common)
    push    %rax
    push    %rbx
    push    %rcx
    push    %rdx
    push    %rdi
    push    %rsi
    push    %rbp
    push    %r8
    push    %r9
    push    %r10
    push    %r11
    push    %r12
    push    %r13
    push    %r14
    push    %r15

    // call C++ routine
    mov     %rsp, %rdi
    call    interrupt_handler

    // restore application state
    mov     %rax, %rsp
    pop     %r15
    pop     %r14
    pop     %r13
    pop     %r12
    pop     %r11
    pop     %r10
    pop     %r9
    pop     %r8
    pop     %rbp
    pop     %rsi
    pop     %rdi
    pop     %rdx
    pop     %rcx
    pop     %rbx
    pop     %rax

    // remove error-code and interrupt-number from stack and return
    add     $16,%rsp
    iretq
END_FUNC(isr_common)

// macro to build a default-isr-handler
.macro BUILD_DEF_ISR no
    BEGIN_FUNC(isr_\no)
    // interrupts are already disabled here since its a interrupt-gate, not a trap-gate
    push    $0                      // error-code (no error here)
    push    $\no                    // the interrupt-number
    jmp     isr_common
    END_FUNC(isr_\no)
.endm

// macro to build an error-isr-handler
.macro BUILD_ERR_ISR no
    BEGIN_FUNC(isr_\no)
    // interrupts are already disabled here since its a interrupt-gate, not a trap-gate
    // the error-code has already been pushed
    push    $\no                    // the interrupt-number
    jmp     isr_common
    END_FUNC(isr_\no)
.endm

// our ISRs
BUILD_DEF_ISR 0
BUILD_DEF_ISR 1
BUILD_DEF_ISR 2
BUILD_DEF_ISR 3
BUILD_DEF_ISR 4
BUILD_DEF_ISR 5
BUILD_DEF_ISR 6
BUILD_DEF_ISR 7
BUILD_ERR_ISR 8
BUILD_DEF_ISR 9
BUILD_ERR_ISR 10
BUILD_ERR_ISR 11
BUILD_ERR_ISR 12
BUILD_ERR_ISR 13
BUILD_ERR_ISR 14
BUILD_DEF_ISR 15
BUILD_DEF_ISR 16
BUILD_DEF_ISR 64

BEGIN_FUNC(isr_null)
    push    $0                      // error-code (no error here)
    push    $32                     // the interrupt-number
    jmp     isr_common
END_FUNC(isr_null)

BEGIN_FUNC(gem5_writefile)
    .long   0x004F040F
    ret
END_FUNC(gem5_writefile)

.bss

    .align 16
    .space 1024, 0
rctmux_stack:
    .space 128, 0
idle_stack:
