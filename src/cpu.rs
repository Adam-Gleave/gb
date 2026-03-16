mod interrupts;
mod opcodes;
mod operands;

use bitflags::bitflags;

use crate::Bus;
use crate::Cartridge;
pub use interrupts::Interrupts;

bitflags! {
    #[derive(Default, Clone, Copy)]
    pub struct Flags: u8 {
        const Z = 0b1000_0000;
        const N = 0b0100_0000;
        const H = 0b0010_0000;
        const C = 0b0001_0000;
    }
}

pub struct Cpu {
    halted: bool,
    cycles: u128,
    opcode: u8,
    sp: u16,
    pc: u16,
    a: u8,
    f: Flags,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
    ei_requested: bool,
    ime: bool,
    bus: Bus,
}

impl Default for Cpu {
    fn default() -> Self {
        Self {
            halted: false,
            cycles: 0,
            opcode: 0x00,
            sp: 0xFFFE,
            pc: Self::ENTRY_POINT,
            a: 0x01,
            f: Flags::from_bits_truncate(0xB0),
            b: 0x00,
            c: 0x13,
            d: 0x00,
            e: 0xD8,
            h: 0x01,
            l: 0x4D,
            ei_requested: false,
            ime: false,
            bus: Bus::default(),
        }
    }
}

impl Cpu {
    pub const ENTRY_POINT: u16 = 0x100;

    pub fn load_cart(&mut self, cart: Cartridge) {
        *self = Default::default();
        self.bus = Bus::from(cart);
    }

    pub fn step(&mut self) {
        if self.halted {
            self.cycle();
            self.handle_interrupt();
            return;
        }

        self.handle_ei_request();

        if self.handle_interrupt() {
            return;
        }

        self.fetch_opcode(self.pc);
        self.decode_execute();

        self.log_state();
    }

    pub fn log_state(&self) {
        log::debug!(
            "A:{:02X} F:{:02X} B:{:02X} C:{:02X} D:{:02X} E:{:02X} H:{:02X} L:{:02X} SP:{:04X} PC:{:04X} PCMEM:{:02X},{:02X},{:02X},{:02X}",
            self.a,
            self.f.bits(),
            self.b,
            self.c,
            self.d,
            self.e,
            self.h,
            self.l,
            self.sp,
            self.pc,
            self.bus.read(self.pc),
            self.bus.read(self.pc + 1),
            self.bus.read(self.pc + 2),
            self.bus.read(self.pc + 3)
        );
    }

    fn handle_ei_request(&mut self) {
        if self.ei_requested {
            self.ei_requested = false;
            self.ime = true;
        }
    }

    fn fetch_opcode(&mut self, addr: u16) {
        self.opcode = self.read_cycle(addr);
        self.pc = self.pc.wrapping_add(1);
    }

    fn cycle(&mut self) {
        self.bus.sync();
        self.cycles = self.cycles.wrapping_add(1);
    }

    fn read_cycle(&mut self, addr: u16) -> u8 {
        self.bus.sync();
        self.cycles = self.cycles.wrapping_add(1);
        self.bus.read(addr)
    }

    fn read_cycle_hi(&mut self, addr: u8) -> u8 {
        let addr_hi = 0xFF00 + addr as u16;
        self.read_cycle(addr_hi)
    }

    fn write_cycle(&mut self, addr: u16, value: u8) {
        self.bus.sync();
        self.cycles = self.cycles.wrapping_add(1);
        self.bus.write(addr, value);
    }

    fn write_cycle_hi(&mut self, addr: u8, value: u8) {
        let addr_hi = 0xFF00 | addr as u16;
        self.write_cycle(addr_hi, value);
    }

    fn fetch_imm8(&mut self) -> u8 {
        let value = self.read_cycle(self.pc);
        self.pc = self.pc.wrapping_add(1);
        value
    }

    fn fetch_imm16(&mut self) -> u16 {
        let lo = self.fetch_imm8();
        let hi = self.fetch_imm8();
        u16::from_le_bytes([lo, hi])
    }

    fn load_r16(hi: u8, lo: u8) -> u16 {
        u16::from_le_bytes([lo, hi])
    }

    fn store_r16(hi: &mut u8, lo: &mut u8, value: u16) {
        let [value_lo, value_hi] = value.to_le_bytes();
        *lo = value_lo;
        *hi = value_hi;
    }

    fn try_set_z(&mut self, value: u8) {
        self.f.set(Flags::Z, value == 0);
    }
}
