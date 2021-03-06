/*
 * Copyright (C) 2016-2018, Nils Asmussen <nils@os.inf.tu-dresden.de>
 * Economic rights: Technische Universitaet Dresden (Germany)
 *
 * This file is part of M3 (Microkernel-based SysteM for Heterogeneous Manycores).
 *
 * M3 is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License version 2 as
 * published by the Free Software Foundation.
 *
 * M3 is distributed in the hope that it will be useful, but
 * WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
 * General Public License version 2 for more details.
 */

#pragma once

#define CAP_TOTAL           512
#define FS_IMG_OFFSET       0x0

#define PAGE_BITS           12
#define PAGE_SIZE           (static_cast<size_t>(1) << PAGE_BITS)
#define PAGE_MASK           (PAGE_SIZE - 1)

#define MOD_HEAP_SIZE       (4 * 1024 * 1024)
#define APP_HEAP_SIZE       (64 * 1024 * 1024)

#define HEAP_SIZE           0x800000
#define EP_COUNT            16

#define RT_START            0x6000
#define RT_SIZE             0x2000
#define RT_END              (RT_START + RT_SIZE)

#define RCTMUX_ENTRY        0x1000
#define RCTMUX_YIELD        0x5FF0
#define RCTMUX_FLAGS        0x5FF8

#define STACK_SIZE          0x8000
#define STACK_BOTTOM        (RT_END + 0x1000)
#define STACK_TOP           (RT_END + STACK_SIZE)

#define MAX_RB_SIZE         32

#define RECVBUF_SPACE       0x3FC00000
#define RECVBUF_SIZE        (4U * PAGE_SIZE)
#define RECVBUF_SIZE_SPM    16384U

// this has to be large enough for forwarded memory reads
#define SYSC_RBUF_ORDER     9
#define SYSC_RBUF_SIZE      (1 << SYSC_RBUF_ORDER)
#define SYSC_RBUF           RECVBUF_SPACE

#define UPCALL_RBUF_ORDER   9
#define UPCALL_RBUF_SIZE    (1 << UPCALL_RBUF_ORDER)
#define UPCALL_RBUF         (SYSC_RBUF + SYSC_RBUF_SIZE)

#define DEF_RBUF_ORDER      8
#define DEF_RBUF_SIZE       (1 << DEF_RBUF_ORDER)
#define DEF_RBUF            (UPCALL_RBUF + UPCALL_RBUF_SIZE)

#define MEMCAP_END          RECVBUF_SPACE
