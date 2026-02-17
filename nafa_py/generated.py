from __future__ import annotations
from typing import Required, TypedDict

class Registers(TypedDict, total=False):
    slrs: Required[list[RegistersPerSlr]]

class RegistersPerSlr(TypedDict, total=False):
    ctl0: Required[int]
    stat: Required[int]
    cor0: Required[int]
    idcode: Required[int]
    axss: Required[int]
    cor1: Required[int]
    wbstar: Required[int]
    timer: Required[int]
    bootsts: Required[int]
    ctl1: Required[int]
    bspi: Required[int]

class S7(TypedDict, total=False):
    jtag: Required[S7Jtag]
    registers: Required[Registers]

class S7Jtag(TypedDict, total=False):
    device: Required[S7JtagPerDevice]
    slrs: Required[list[S7JtagPerSlr]]

class S7JtagPerDevice(TypedDict, total=False):
    cntl: Required[list[int]]

class S7JtagPerSlr(TypedDict, total=False):
    idcode: Required[list[int]]
    usercode: Required[list[int]]
    fuse_dna: Required[list[int]]
    fuse_key: Required[list[int]]
    fuse_user: Required[list[int]]
    user1: Required[list[int]]
    user2: Required[list[int]]
    user3: Required[list[int]]
    user4: Required[list[int]]

class UP(TypedDict, total=False):
    jtag: Required[UPJtag]
    registers: Required[Registers]

class UPJtag(TypedDict, total=False):
    device: Required[UPJtagPerDevice]
    slrs: Required[list[USJtagPerSlr]]

class UPJtagPerDevice(TypedDict, total=False):
    cntl: Required[list[int]]

class US(TypedDict, total=False):
    jtag: Required[USJtag]
    registers: Required[Registers]

class USJtag(TypedDict, total=False):
    device: Required[USJtagPerDevice]
    slrs: Required[list[USJtagPerSlr]]

class USJtagPerDevice(TypedDict, total=False):
    cntl: Required[list[int]]

class USJtagPerSlr(TypedDict, total=False):
    idcode: Required[list[int]]
    usercode: Required[list[int]]
    fuse_dna: Required[list[int]]
    fuse_key: Required[list[int]]
    fuse_user: Required[list[int]]
    fuse_user_128: Required[list[int]]
    fuse_rsa: Required[list[int]]
    fuse_sec: Required[list[int]]
    user1: Required[list[int]]
    user2: Required[list[int]]
    user3: Required[list[int]]
    user4: Required[list[int]]

class ZP(TypedDict, total=False):
    jtag: Required[UPJtag]
    registers: Required[Registers]

class _Ignore(TypedDict, total=False):
    s7: Required[S7]
    up: Required[UP]
    us: Required[US]
    zp: Required[ZP]


