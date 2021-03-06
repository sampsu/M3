/**
* Copyright (C) 2016-2018, Nils Asmussen <nils@os.inf.tu-dresden.de>
* Economic rights: Technische Universität Dresden (Germany)
*
* This file is part of M3 (Microkernel for Minimalist Manycores).
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

#include <base/stream/IStringStream.h>

#include <m3/com/GateStream.h>
#include <m3/server/SimpleRequestHandler.h>
#include <m3/server/Server.h>

using namespace m3;

enum TestOp {
    TEST
};

class TestRequestHandler;
using base_class = SimpleRequestHandler<TestRequestHandler, TestOp, 1>;

class TestRequestHandler : public base_class {
public:
    explicit TestRequestHandler()
        : base_class(),
          _cnt() {
        add_operation(TEST, &TestRequestHandler::test);
    }

    void test(GateIStream &is) {
        reply_vmsg(is, _cnt++);
    }

private:
    int _cnt;
};

int main(int argc, char **argv) {
    Server<TestRequestHandler> *srv;
    if(argc > 1) {
        String input(argv[1]);
        IStringStream is(input);
        capsel_t sels;
        epid_t ep;
        is >> sels >> ep;

        srv = new Server<TestRequestHandler>(sels, ep, new TestRequestHandler());
    }
    else
        srv = new Server<TestRequestHandler>("test", new TestRequestHandler());

    env()->workloop()->run();

    delete srv;
    return 0;
}
