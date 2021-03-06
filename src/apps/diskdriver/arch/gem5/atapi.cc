/*
 * Copyright (C) 2017, Lukas Landgraf <llandgraf317@gmail.com>
 * Copyright (C) 2018, Nils Asmussen <nils@os.inf.tu-dresden.de>
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

/* Changes: October 2017 by Lukas Landgraf, Email: llandgraf317@gmail.com
 * This is mostly legacy code copied from Escape OS by Nils Asmussen. It is currently
 * not used for actions in the driver. Logging is modified for M3.
 */

#include "atapi.h"

#include "ata.h"
#include "controller.h"
#include "device.h"

using namespace m3;

static bool atapi_request(sATADevice *device, MemGate &mem, size_t offset, size_t bufSize);

void atapi_softReset(sATADevice *device) {
    int i = 1000000;
    ctrl_outb(device->ctrl, ATA_REG_DRIVE_SELECT, ((device->id & SLAVE_BIT) << 4) | 0xA0);
    ctrl_wait(device->ctrl);
    ctrl_outb(device->ctrl, ATA_REG_COMMAND, COMMAND_ATAPI_RESET);
    while((ctrl_inb(device->ctrl, ATA_REG_STATUS) & CMD_ST_BUSY) && i--)
        ctrl_wait(device->ctrl);
    ctrl_outb(device->ctrl, ATA_REG_DRIVE_SELECT, (device->id << 4) | 0xA0);
    i = 1000000;
    while((ctrl_inb(device->ctrl, ATA_REG_CONTROL) & CMD_ST_BUSY) && i--)
        ctrl_wait(device->ctrl);
    ctrl_wait(device->ctrl);
}

bool atapi_read(sATADevice *device, uint op, MemGate &mem, size_t offset, uint64_t lba,
                UNUSED size_t secSize, size_t secCount) {
    uint8_t cmd[] = {SCSI_CMD_READ_SECTORS_EXT, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0};
    if(!device->info.feats.flags.lba48)
        cmd[0] = SCSI_CMD_READ_SECTORS;
    /* no writing here ;) */
    if(op != OP_READ)
        return false;
    if(secCount == 0)
        return false;
    if(cmd[0] == SCSI_CMD_READ_SECTORS_EXT) {
        cmd[6] = (secCount >> 24) & 0xFF;
        cmd[7] = (secCount >> 16) & 0xFF;
        cmd[8] = (secCount >> 8) & 0xFF;
        cmd[9] = (secCount >> 0) & 0xFF;
    }
    else {
        cmd[7] = (secCount >> 8) & 0xFF;
        cmd[8] = (secCount >> 0) & 0xFF;
    }
    cmd[2] = (lba >> 24) & 0xFF;
    cmd[3] = (lba >> 16) & 0xFF;
    cmd[4] = (lba >> 8) & 0xFF;
    cmd[5] = (lba >> 0) & 0xFF;
    mem.write(cmd, sizeof(cmd), 0);

    return atapi_request(device, mem, offset, secCount * device->secSize);
}

size_t atapi_getCapacity(sATADevice *device) {
    MemGate temp = MemGate::create_global(8 + sizeof(sPRD), MemGate::RW);
    ctrl_setupDMA(temp);

    uint8_t cmd[] = {SCSI_CMD_READ_CAPACITY, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0};
    temp.write(cmd, sizeof(cmd), 0);
    bool res      = atapi_request(device, temp, 0, 8);
    if(!res)
        return 0;

    uint8_t resp[8];
    temp.read(&resp, sizeof(resp), 0);
    return ((size_t)resp[0] << 24) | ((size_t)resp[1] << 16) | ((size_t)resp[2] << 8) | resp[3];
}

static bool atapi_request(sATADevice *device, MemGate &mem, size_t offset, size_t bufSize) {
    int res;
    size_t size;
    sATAController *ctrl = device->ctrl;

    /* send PACKET command to drive */
    if(!ata_readWrite(device, OP_PACKET, mem, offset, 0xFFFF00, 12, 1))
        return false;

    /* now transfer the data */
    if(ctrl->useDma && device->info.caps.flags.DMA)
        return ata_transferDMA(device, OP_READ, mem, offset, device->secSize, bufSize / device->secSize);

    /* ok, no DMA, so wait first until the drive is ready */
    res = ctrl_waitUntil(ctrl, ATAPI_TRANSFER_TIMEOUT, ATAPI_TRANSFER_SLEEPTIME, CMD_ST_DRQ, CMD_ST_BUSY);
    if(res == -1) {
        SLOG(IDE, "Device " << device->id << ": Timeout before ATAPI-PIO-transfer");
        return false;
    }
    if(res != 0) {
        SLOG(IDE, "Device " << device->id << ": ATAPI-PIO-transfer failed: " << res);
        return false;
    }

    /* read the actual size per transfer */
    SLOG(IDE_ALL, "Reading response-size");
    size = ((size_t)ctrl_inb(ctrl, ATA_REG_ADDRESS3) << 8) | (size_t)ctrl_inb(ctrl, ATA_REG_ADDRESS2);
    /* do the PIO-transfer (no check at the beginning; seems to cause trouble on some machines) */
    return ata_transferPIO(device, OP_READ, mem, offset, size, bufSize / size, false);
}
