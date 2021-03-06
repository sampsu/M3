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

#include <base/Common.h>
#include <base/util/Util.h>
#include <base/WorkLoop.h>
#include <base/DTU.h>

#include <m3/com/Gate.h>

#include <functional>

namespace m3 {

class GateIStream;
class SendGate;
class VPE;

/**
 * A receive gate is used to receive messages from send gates. To this end, it has a receive buffer
 * of a fixed message and total size. Multiple send gates can be created for one receive gate. After
 * a message has been received, the reply operation can be used to send a reply back to the sender.
 *
 * Receiving messages is possible by waiting for them using the wait() method. This approach is used
 * when, e.g., receiving a reply upon a sent message. Alternatively, one can start to listen to
 * received messages. In this case, a WorkLoop item is created.
 */
class RecvGate : public Gate {
    enum {
        FREE_BUF    = 1,
        FREE_EP     = 2,
    };

    class RecvGateWorkItem : public WorkItem {
    public:
        explicit RecvGateWorkItem(RecvGate *buf) : _buf(buf) {
        }

        virtual void work() override;

    protected:
        RecvGate *_buf;
    };

    explicit RecvGate(VPE &vpe, capsel_t cap, int order, uint flags)
        : Gate(RECV_GATE, cap, flags),
          _vpe(vpe),
          _buf(),
          _order(order),
          _free(FREE_BUF),
          _handler(),
          _workitem() {
    }
    explicit RecvGate(VPE &vpe, capsel_t cap, epid_t ep, void *buf, int order, int msgorder, uint flags);

public:
    using msghandler_t = std::function<void(GateIStream&)>;

    /**
     * @return the receive gate for system call replies
     */
    static RecvGate &syscall() {
        return _syscall;
    }
    /**
     * @return the receive gate for upcalls
     */
    static RecvGate &upcall() {
        return _upcall;
    }
    /**
     * @return the default receive gate. can be used whenever a buffer for a single message with a
     *  reasonable size is sufficient
     */
    static RecvGate &def() {
        return _default;
    }

    /**
     * Creates a new receive gate with given size.
     *
     * @param order the size of the buffer (2^<order> bytes)
     * @param msgorder the size of messages within the buffer (2^<msgorder> bytes)
     * @return the receive gate
     */
    static RecvGate create(int order, int msgorder);
    /**
     * Creates a new receive gate at selector <sel> with given size.
     *
     * @param sel the capability selector to use
     * @param order the size of the buffer (2^<order> bytes)
     * @param msgorder the size of messages within the buffer (2^<msgorder> bytes)
     * @return the receive gate
     */
    static RecvGate create(capsel_t sel, int order, int msgorder);

    /**
     * Creates a new receive gate that should be activated for <vpe>.
     *
     * @param vpe the VPE that should activate the receive gate
     * @param order the size of the buffer (2^<order> bytes)
     * @param msgorder the size of messages within the buffer (2^<msgorder> bytes)
     * @return the receive gate
     */
    static RecvGate create_for(VPE &vpe, int order, int msgorder);
    /**
     * Creates a new receive gate at selector <sel> that should be activated for <vpe>.
     *
     * @param vpe the VPE that should activate the receive gate
     * @param sel the capability selector to use
     * @param order the size of the buffer (2^<order> bytes)
     * @param msgorder the size of messages within the buffer (2^<msgorder> bytes)
     * @return the receive gate
     */
    static RecvGate create_for(VPE &vpe, capsel_t sel, int order, int msgorder);

    /**
     * Binds the receive gate at selector <sel>.
     *
     * @param sel the capability selector
     * @param order the size of the buffer (2^<order> bytes)
     * @param ep the endpoint it has already been activated to
     * @return the receive gate
     */
    static RecvGate bind(capsel_t sel, int order, epid_t ep = EP_COUNT);

    RecvGate(const RecvGate&) = delete;
    RecvGate &operator=(const RecvGate&) = delete;
    RecvGate(RecvGate &&r)
            : Gate(Util::move(r)),
              _vpe(r._vpe),
              _buf(r._buf),
              _order(r._order),
              _free(r._free),
              _handler(r._handler),
              _workitem(r._workitem) {
        r._free = 0;
        r._workitem = nullptr;
    }
    ~RecvGate();

    /**
     * @return the buffer address
     */
    const void *addr() const {
        return _buf;
    }

    /**
     * Activates this receive gate, i.e., lets the kernel configure a free endpoint for it
     */
    void activate();
    /**
     * Activates this receive gate, i.e., lets the kernel configure endpoint <ep> for it
     */
    void activate(epid_t ep);
    /**
     * Activates this receive gate, i.e., lets the kernel configure endpoint <ep> for it and use
     * <addr> as the buffer.
     */
    void activate(epid_t ep, uintptr_t addr);

    /**
     * Deactivates and stops the receive gate.
     */
    void deactivate();

    /**
     * Starts to listen for received messages, i.e., creates a WorkLoop item.
     *
     * @param handler the handler to call for received messages
     */
    void start(msghandler_t handler);

    /**
     * Stops to listen for received messages
     */
    void stop();

    /**
     * Fetches a message from this receive gate and returns it, if there is any.
     *
     * @return the message or nullptr
     */
    const DTU::Message *fetch() {
        activate();
        return DTU::get().fetch_msg(ep());
    }

    /**
     * Waits until a message is received. If <sgate> is given, it will stop if as soon as <sgate>
     * gets invalid and return the appropriate error.
     *
     * @param sgate the send gate (optional), if waiting for a reply
     * @param msg will be set to the fetched message
     * @return the error code
     */
    Errors::Code wait(SendGate *sgate, const DTU::Message **msg);

    /**
     * Replies the <len> bytes at <data> to the message at <msgidx>.
     *
     * @param data the data to send
     * @param len the length of the data
     * @param msgidx the index of the message to reply to
     * @return the error code or Errors::NONE
     */
    Errors::Code reply(const void *data, size_t len, size_t msgidx);

    /**
     * Drops all messages with given label. That is, these messages will be marked as read.
     *
     * @param label the label
     */
    void drop_msgs_with(label_t label) {
        DTU::get().drop_msgs(ep(), label);
    }

private:
    static void *allocate(VPE &vpe, epid_t ep, size_t size);
    static void free(void *);

    VPE &_vpe;
    void *_buf;
    int _order;
    uint _free;
    msghandler_t _handler;
    RecvGateWorkItem *_workitem;
    static RecvGate _syscall;
    static RecvGate _upcall;
    static RecvGate _default;
};

}
