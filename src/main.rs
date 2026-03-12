use std::fs::File;
use std::io;
use std::io::BufReader;

use bitflags::bitflags;
use clap::Parser;

trait SrcOperand8 {
    fn read(&self, cpu: &mut Cpu) -> u8;
}

trait DstOperand8 {
    fn write(&self, cpu: &mut Cpu, value: u8);
}

trait SrcOperand16 {
    fn read(&self, cpu: &mut Cpu) -> u16;
}

trait DstOperand16 {
    fn write(&self, cpu: &mut Cpu, value: u16);
}

#[derive(Clone, Copy, Debug)]
enum Register {
    A,
    F,
    B,
    C,
    D,
    E,
    H,
    L,
}

impl SrcOperand8 for Register {
    fn read(&self, cpu: &mut Cpu) -> u8 {
        match self {
            Self::A => cpu.a,
            Self::F => cpu.f.bits(),
            Self::B => cpu.b,
            Self::C => cpu.c,
            Self::D => cpu.d,
            Self::E => cpu.e,
            Self::H => cpu.h,
            Self::L => cpu.l,
        }
    }
}

impl DstOperand8 for Register {
    fn write(&self, cpu: &mut Cpu, value: u8) {
        match self {
            Self::A => cpu.a = value,
            Self::F => cpu.f = Flags::from_bits_truncate(value),
            Self::B => cpu.b = value,
            Self::C => cpu.c = value,
            Self::D => cpu.d = value,
            Self::E => cpu.e = value,
            Self::H => cpu.h = value,
            Self::L => cpu.l = value,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum RegisterPair {
    AF,
    BC,
    DE,
    HL,
    SP,
}

impl SrcOperand16 for RegisterPair {
    fn read(&self, cpu: &mut Cpu) -> u16 {
        match self {
            Self::AF => Cpu::load_r16(cpu.a, cpu.f.bits()),
            Self::BC => Cpu::load_r16(cpu.b, cpu.c),
            Self::DE => Cpu::load_r16(cpu.d, cpu.e),
            Self::HL => Cpu::load_r16(cpu.h, cpu.l),
            Self::SP => cpu.sp,
        }
    }
}

impl DstOperand16 for RegisterPair {
    fn write(&self, cpu: &mut Cpu, value: u16) {
        match self {
            Self::AF => {
                let mut f = 0;
                Cpu::store_r16(&mut cpu.a, &mut f, value);
                cpu.f = Flags::from_bits_truncate(f);
            }
            Self::BC => Cpu::store_r16(&mut cpu.b, &mut cpu.c, value),
            Self::DE => Cpu::store_r16(&mut cpu.d, &mut cpu.e, value),
            Self::HL => Cpu::store_r16(&mut cpu.h, &mut cpu.l, value),
            Self::SP => cpu.sp = value,
        }
    }
}

#[derive(Clone, Copy)]
struct Imm8;

impl SrcOperand8 for Imm8 {
    fn read(&self, cpu: &mut Cpu) -> u8 {
        cpu.fetch_imm8()
    }
}

#[derive(Clone, Copy)]
struct Imm16;

impl SrcOperand16 for Imm16 {
    fn read(&self, cpu: &mut Cpu) -> u16 {
        cpu.fetch_imm16()
    }
}

#[derive(Clone, Copy)]
struct Ind8;

impl SrcOperand8 for Ind8 {
    fn read(&self, cpu: &mut Cpu) -> u8 {
        let addr_offset = cpu.fetch_imm8();
        cpu.read_cycle_hi(addr_offset)
    }
}

impl DstOperand8 for Ind8 {
    fn write(&self, cpu: &mut Cpu, value: u8) {
        let addr_offset = cpu.fetch_imm8();
        cpu.write_cycle_hi(addr_offset, value);
    }
}

#[derive(Clone, Copy)]
struct Ind16;

impl SrcOperand8 for Ind16 {
    fn read(&self, cpu: &mut Cpu) -> u8 {
        let addr = cpu.fetch_imm16();
        cpu.read_cycle(addr)
    }
}

impl DstOperand8 for Ind16 {
    fn write(&self, cpu: &mut Cpu, value: u8) {
        let addr = cpu.fetch_imm16();
        cpu.write_cycle(addr, value);
    }
}

#[derive(Clone, Copy)]
struct Addr16;

impl SrcOperand16 for Addr16 {
    fn read(&self, cpu: &mut Cpu) -> u16 {
        cpu.fetch_imm16()
    }
}

#[derive(Clone, Copy)]
enum RegisterPtr {
    C,
    BC,
    DE,
    HL,
    HLD,
    HLI,
}

impl SrcOperand8 for RegisterPtr {
    fn read(&self, cpu: &mut Cpu) -> u8 {
        let (hi, lo) = match self {
            Self::C => (0, cpu.c),
            Self::BC => (cpu.b, cpu.c),
            Self::DE => (cpu.d, cpu.e),
            Self::HL | Self::HLD | Self::HLI => (cpu.h, cpu.l),
        };
        let addr = u16::from_le_bytes([lo, hi]);
        let value = cpu.read_cycle(addr);

        match self {
            Self::HLD => {
                let addr = addr.wrapping_sub(1);
                Cpu::store_r16(&mut cpu.h, &mut cpu.l, addr);
            }
            Self::HLI => {
                let addr = addr.wrapping_add(1);
                Cpu::store_r16(&mut cpu.h, &mut cpu.l, addr);
            }
            _ => {}
        }

        value
    }
}

impl DstOperand8 for RegisterPtr {
    fn write(&self, cpu: &mut Cpu, value: u8) {
        let (hi, lo) = match self {
            Self::C => (0, cpu.c),
            Self::BC => (cpu.b, cpu.c),
            Self::DE => (cpu.d, cpu.e),
            Self::HL | Self::HLD | Self::HLI => (cpu.h, cpu.l),
        };
        let addr = u16::from_le_bytes([lo, hi]);
        cpu.write_cycle(addr, value);

        match self {
            Self::HLD => {
                let addr = addr.wrapping_sub(1);
                Cpu::store_r16(&mut cpu.h, &mut cpu.l, addr);
            }
            Self::HLI => {
                let addr = addr.wrapping_add(1);
                Cpu::store_r16(&mut cpu.h, &mut cpu.l, addr);
            }
            _ => {}
        }
    }
}

bitflags! {
    #[derive(Default, Clone, Copy)]
    pub struct Flags: u8 {
        const Z = 0b1000_0000;
        const N = 0b0100_0000;
        const H = 0b0010_0000;
        const C = 0b0001_0000;
    }
}

pub const ENTRY_POINT: u16 = 0x100;

#[derive(Default)]
pub struct Pc(pub u16);

impl Pc {
    pub fn get(&self) -> u16 {
        self.0
    }

    pub fn set(&mut self, addr: u16) {
        self.0 = addr;
    }

    pub fn inc(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }

    pub fn dec(&mut self) {
        self.0 = self.0.wrapping_sub(1);
    }
}

#[derive(Default)]
pub struct Cpu {
    cycles: u128,

    opcode: u8,
    sp: u16,
    pc: Pc,
    a: u8,
    f: Flags,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
    ime: bool,

    bus: Bus,
}

impl Cpu {
    pub fn load_cart(&mut self, cart: Cartridge) {
        *self = Default::default();
        self.bus = Bus::from(cart);
        self.pc = Pc(ENTRY_POINT);
    }

    pub fn decode_execute(&mut self) {
        print!("\nOpcode: {:#04X}", self.opcode);

        match self.opcode {
            0x00 => self.noop(),
            0x01 => self.load_16_16(RegisterPair::BC, Imm16),
            0x03 => self.inc_16(RegisterPair::BC),
            0x04 => self.inc_8(Register::B),
            0x05 => self.dec_8(Register::B),
            0x06 => self.load_8_8(Register::B, Imm8),
            0x0A => self.load_8_8(Register::A, RegisterPtr::BC),
            0x0B => self.dec_16(RegisterPair::BC),
            0x0C => self.inc_8(Register::C),
            0x0D => self.dec_8(Register::C),
            0x0E => self.load_8_8(Register::C, Imm8),
            0x11 => self.load_16_16(RegisterPair::DE, Imm16),
            0x13 => self.inc_16(RegisterPair::DE),
            0x14 => self.inc_8(Register::D),
            0x15 => self.dec_8(Register::D),
            0x16 => self.load_8_8(Register::D, Imm8),
            0x1A => self.load_8_8(Register::A, RegisterPtr::DE),
            0x1B => self.dec_16(RegisterPair::DE),
            0x1C => self.inc_8(Register::E),
            0x1D => self.dec_8(Register::E),
            0x20 => self.jr_nz(),
            0x21 => self.load_16_16(RegisterPair::HL, Imm16),
            0x22 => self.load_8_8(RegisterPtr::HLI, Register::A),
            0x23 => self.inc_16(RegisterPair::HL),
            0x24 => self.inc_8(Register::H),
            0x25 => self.dec_8(Register::H),
            0x26 => self.load_8_8(Register::H, Imm8),
            0x2A => self.load_8_8(Register::A, RegisterPtr::HLI),
            0x2B => self.dec_16(RegisterPair::HL),
            0x2C => self.inc_8(Register::L),
            0x2D => self.dec_8(Register::L),
            0x2F => self.cpl(),
            0x31 => self.load_16_16(RegisterPair::SP, Imm16),
            0x32 => self.load_8_8(RegisterPtr::HLD, Register::A),
            0x33 => self.inc_16(RegisterPair::SP),
            0x34 => self.inc_8(RegisterPtr::HL),
            0x35 => self.dec_8(RegisterPtr::HL),
            0x36 => self.load_8_8(RegisterPtr::HL, Imm8),
            0x3A => self.load_8_8(Register::A, RegisterPtr::HLD),
            0x3B => self.dec_16(RegisterPair::SP),
            0x3C => self.inc_8(Register::A),
            0x3D => self.dec_8(Register::A),
            0x3E => self.load_8_8(Register::A, Imm8),
            0x40 => self.load_8_8(Register::B, Register::B),
            0x41 => self.load_8_8(Register::B, Register::C),
            0x42 => self.load_8_8(Register::B, Register::D),
            0x43 => self.load_8_8(Register::B, Register::E),
            0x44 => self.load_8_8(Register::B, Register::H),
            0x45 => self.load_8_8(Register::B, Register::L),
            0x46 => self.load_8_8(Register::B, RegisterPtr::HL),
            0x47 => self.load_8_8(Register::B, Register::A),
            0x48 => self.load_8_8(Register::C, Register::B),
            0x49 => self.load_8_8(Register::C, Register::C),
            0x4A => self.load_8_8(Register::C, Register::D),
            0x4B => self.load_8_8(Register::C, Register::E),
            0x4C => self.load_8_8(Register::C, Register::H),
            0x4D => self.load_8_8(Register::C, Register::L),
            0x4E => self.load_8_8(Register::C, RegisterPtr::HL),
            0x4F => self.load_8_8(Register::C, Register::A),
            0x50 => self.load_8_8(Register::D, Register::B),
            0x51 => self.load_8_8(Register::D, Register::C),
            0x52 => self.load_8_8(Register::D, Register::D),
            0x53 => self.load_8_8(Register::D, Register::E),
            0x54 => self.load_8_8(Register::D, Register::H),
            0x55 => self.load_8_8(Register::D, Register::L),
            0x56 => self.load_8_8(Register::D, RegisterPtr::HL),
            0x57 => self.load_8_8(Register::D, Register::A),
            0x58 => self.load_8_8(Register::E, Register::B),
            0x59 => self.load_8_8(Register::E, Register::C),
            0x5A => self.load_8_8(Register::E, Register::D),
            0x5B => self.load_8_8(Register::E, Register::E),
            0x5C => self.load_8_8(Register::E, Register::H),
            0x5D => self.load_8_8(Register::E, Register::L),
            0x5E => self.load_8_8(Register::E, RegisterPtr::HL),
            0x5F => self.load_8_8(Register::E, Register::A),
            0x60 => self.load_8_8(Register::H, Register::B),
            0x61 => self.load_8_8(Register::H, Register::C),
            0x62 => self.load_8_8(Register::H, Register::D),
            0x63 => self.load_8_8(Register::H, Register::E),
            0x64 => self.load_8_8(Register::H, Register::H),
            0x65 => self.load_8_8(Register::H, Register::L),
            0x66 => self.load_8_8(Register::H, RegisterPtr::HL),
            0x67 => self.load_8_8(Register::H, Register::A),
            0x68 => self.load_8_8(Register::L, Register::B),
            0x69 => self.load_8_8(Register::L, Register::C),
            0x6A => self.load_8_8(Register::L, Register::D),
            0x6B => self.load_8_8(Register::L, Register::E),
            0x6C => self.load_8_8(Register::L, Register::H),
            0x6D => self.load_8_8(Register::L, Register::L),
            0x6E => self.load_8_8(Register::L, RegisterPtr::HL),
            0x6F => self.load_8_8(Register::L, Register::A),
            0x70 => self.load_8_8(RegisterPtr::HL, Register::B),
            0x71 => self.load_8_8(RegisterPtr::HL, Register::C),
            0x72 => self.load_8_8(RegisterPtr::HL, Register::D),
            0x73 => self.load_8_8(RegisterPtr::HL, Register::E),
            0x74 => self.load_8_8(RegisterPtr::HL, Register::H),
            0x75 => self.load_8_8(RegisterPtr::HL, Register::L),
            0x76 => self.halt(),
            0x77 => self.load_8_8(RegisterPtr::HL, Register::A),
            0x78 => self.load_8_8(Register::A, Register::B),
            0x79 => self.load_8_8(Register::A, Register::C),
            0x7A => self.load_8_8(Register::A, Register::D),
            0x7B => self.load_8_8(Register::A, Register::E),
            0x7C => self.load_8_8(Register::A, Register::H),
            0x7D => self.load_8_8(Register::A, Register::L),
            0x7E => self.load_8_8(Register::A, RegisterPtr::HL),
            0x7F => self.load_8_8(Register::A, Register::A),
            0xA0 => self.and(Register::B),
            0xA1 => self.and(Register::C),
            0xA2 => self.and(Register::D),
            0xA3 => self.and(Register::E),
            0xA4 => self.and(Register::H),
            0xA5 => self.and(Register::L),
            0xA6 => self.and(RegisterPtr::HL),
            0xA7 => self.and(Register::A),
            0xA8 => self.xor(Register::B),
            0xA9 => self.xor(Register::C),
            0xAA => self.xor(Register::D),
            0xAB => self.xor(Register::E),
            0xAC => self.xor(Register::H),
            0xAD => self.xor(Register::L),
            0xAE => self.xor(RegisterPtr::HL),
            0xAF => self.xor(Register::A),
            0xB0 => self.or(Register::B),
            0xB1 => self.or(Register::C),
            0xB2 => self.or(Register::D),
            0xB3 => self.or(Register::E),
            0xB4 => self.or(Register::H),
            0xB5 => self.or(Register::L),
            0xB6 => self.or(RegisterPtr::HL),
            0xB7 => self.or(Register::A),
            0xB8 => self.cp(Register::A, Register::B),
            0xB9 => self.cp(Register::A, Register::C),
            0xBA => self.cp(Register::A, Register::D),
            0xBB => self.cp(Register::A, Register::E),
            0xBC => self.cp(Register::A, Register::H),
            0xBD => self.cp(Register::A, Register::L),
            0xBE => self.cp(Register::A, RegisterPtr::HL),
            0xBF => self.cp(Register::A, Register::A),
            0xC3 => self.jp(),
            0xCD => self.call_16(Addr16),
            0xC9 => self.ret(),
            0xE0 => self.load_8_8(Ind8, Register::A),
            0xE2 => self.load_8_8(RegisterPtr::C, Register::A),
            0xE6 => self.and(Imm8),
            0xEA => self.load_8_8(Ind16, Register::A),
            0xEE => self.xor(Imm8),
            0xF0 => self.load_8_8(Register::A, Ind8),
            0xF2 => self.load_8_8(Register::A, RegisterPtr::C),
            0xF3 => self.di(),
            0xF6 => self.or(Imm8),
            0xFB => self.ei(),
            0xFE => self.cp(Register::A, Imm8),
            _ => panic!("Unexpected opcode {:#04X}", self.opcode),
        }
    }

    fn prefetch(&mut self, addr: u16) {
        self.opcode = self.read_cycle(addr);
        self.pc.inc();
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
        let addr_hi = 0xFF00 | addr as u16;
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
        let value = self.read_cycle(self.pc.get());
        self.pc.inc();
        value
    }

    fn fetch_imm16(&mut self) -> u16 {
        let lo = self.fetch_imm8();
        let hi = self.fetch_imm8();
        u16::from_le_bytes([lo, hi])
    }

    fn noop(&mut self) {
        self.prefetch(self.pc.get());
    }

    fn halt(&mut self) -> ! {
        unimplemented!()
    } 

    fn load_8_8<Dst: DstOperand8, Src: SrcOperand8>(&mut self, dst: Dst, src: Src) {
        let value = src.read(self);
        dst.write(self, value);
        self.prefetch(self.pc.get());
    }

    fn load_16_16<Dst: DstOperand16, Src: SrcOperand16>(&mut self, dst: Dst, src: Src) {
        let value = src.read(self);
        dst.write(self, value);
        self.prefetch(self.pc.get());
    }

    fn dec_8<O: SrcOperand8 + DstOperand8>(&mut self, operand: O) {
        let value = operand.read(self);
        let result = value.wrapping_sub(1);
        self.try_set_z(result);
        self.f.set(Flags::N, true);
        self.f.set(Flags::H, value & 0xF == 0);
        operand.write(self, result);
        self.prefetch(self.pc.get());
    }

    fn dec_16<O: SrcOperand16 + DstOperand16>(&mut self, operand: O) {
        let value = operand.read(self);
        let value = value.wrapping_sub(1);
        operand.write(self, value);
        self.cycle();
        self.prefetch(self.pc.get());
    }

    fn inc_8<O: SrcOperand8 + DstOperand8>(&mut self, operand: O) {
        let value = operand.read(self);
        let result = value.wrapping_add(1);
        self.try_set_z(result);
        self.f.set(Flags::N, true);
        self.f.set(Flags::H, value & 0xF == 0xF);
        operand.write(self, result);
        self.prefetch(self.pc.get());
    }

    fn inc_16<O: SrcOperand16 + DstOperand16>(&mut self, operand: O) {
        let value = operand.read(self);
        let value = value.wrapping_add(1);
        operand.write(self, value);
        self.cycle();
        self.prefetch(self.pc.get());
    }

    fn jr_nz(&mut self) {
        let offset = self.fetch_imm8();
        if !self.f.contains(Flags::Z) {
            self.do_jr(offset as i8);
        }
        self.prefetch(self.pc.get());
    }

    fn cpl(&mut self) {
        self.a = !self.a;
        self.f.set(Flags::N, true);
        self.f.set(Flags::H, true);
        self.prefetch(self.pc.get());
    }

    fn and<O: SrcOperand8>(&mut self, operand: O) {
        let operand = operand.read(self);
        let value = self.a & operand;
        self.a = value;
        self.try_set_z(value);
        self.f.set(Flags::N, false);
        self.f.set(Flags::H, false);
        self.f.set(Flags::C, false);
        self.prefetch(self.pc.get());
    }

    fn xor<O: SrcOperand8>(&mut self, operand: O) {
        let operand = operand.read(self);
        let value = self.a ^ operand;
        self.a = value;
        self.try_set_z(value);
        self.f.set(Flags::N, false);
        self.f.set(Flags::H, false);
        self.f.set(Flags::C, false);
        self.prefetch(self.pc.get());
    }

    fn or<O: SrcOperand8>(&mut self, operand: O) {
        let operand = operand.read(self);
        let value = self.a | operand;
        self.a = value;
        self.f.set(Flags::N, false);
        self.f.set(Flags::H, false);
        self.f.set(Flags::C, false);
        self.prefetch(self.pc.get());
    }

    fn jp(&mut self) {
        let addr = self.fetch_imm16();
        self.do_jp(addr);
        self.prefetch(self.pc.get());
    }

    fn call_16<O: SrcOperand16>(&mut self, operand: O) {
        let addr = operand.read(self);
        self.do_call(addr);
        self.prefetch(self.pc.get());
    }

    fn ret(&mut self) {
        self.do_ret();
        self.prefetch(self.pc.get());
    }

    fn di(&mut self) {
        self.ime = false;
        self.prefetch(self.pc.get());
    }

    fn ei(&mut self) {
        self.ime = true;
        self.prefetch(self.pc.get());
    }

    fn cp<A: SrcOperand8, B: SrcOperand8>(&mut self, a: A, b: B) {
        let a_value = a.read(self);
        let b_value = b.read(self);
        print!(" CP {}, {}", a_value, b_value);
        let value = b_value.wrapping_sub(a_value);
        self.try_set_z(value);
        self.f.set(Flags::N, true);
        self.f.set(Flags::H, (a_value & 0xF) < (b_value & 0xF));
        self.f.set(Flags::C, a_value < b_value);
        self.prefetch(self.pc.get());
    }

    fn do_jr(&mut self, offset: i8) {
        let addr = self.pc.get().wrapping_add(offset as u16);
        self.pc.set(addr);
        self.cycle();
    }

    fn do_jp(&mut self, addr: u16) {
        self.pc.set(addr);
        self.cycle();
    }

    fn do_call(&mut self, addr: u16) {
        self.do_push_16(self.pc.get());
        self.pc.set(addr);
    }

    fn do_ret(&mut self) {
        let addr = self.do_pop_16();
        self.pc.set(addr);
        self.cycle();
    }

    fn do_push_16(&mut self, value: u16) {
        let [lo, hi] = u16::to_le_bytes(value);
        self.cycle();
        self.sp = self.sp.wrapping_sub(1);
        self.write_cycle(self.sp, hi);
        self.sp = self.sp.wrapping_sub(1);
        self.write_cycle(self.sp, lo);
    }

    fn do_pop_16(&mut self) -> u16 {
        let lo = self.read_cycle(self.sp);
        self.sp = self.sp.wrapping_add(1);
        let hi = self.read_cycle(self.sp);
        self.sp = self.sp.wrapping_add(1);
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

pub struct PpuBus<'a> {
    io: &'a mut IoRegisters,
}

#[derive(Default)]
pub struct Ppu {
    lcd_enable: bool,
    mode: u8,
    dots: u16,
    vram: Memory<0x2000, 0x8000>,
    oam: Memory<0x0100, 0xFE00>,
} 

impl Ppu {
    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0x8000..=0x9FFF if !self.lcd_enable || self.mode != 3 => self.vram.read(addr),
            0xFE00..=0xFE9F if !self.lcd_enable || self.mode <= 1 => self.oam.read(addr),
            _ => {
                println!("Tried to read from PPU-managed address {:#06X}", addr);
                0xFF
            }
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x8000..=0x9FFF if !self.lcd_enable || self.mode != 3 => self.vram.write(addr, value),
            0xFE00..=0xFE9F if !self.lcd_enable || self.mode <= 1 => self.oam.write(addr, value),
            _ => panic!("Tried to write to PPU-managed address {:#06X} in mode {}", addr, self.mode),
        }
    }

    const DOT_CYCLES: usize = 4;
    const SCANLINE_DOTS: u16 = 456;
    const M2_DOTS: u16 = 80;
    const M3_DOTS: u16 = 172;
    const M0_DOTS: u16 = 376 - Self::M3_DOTS;

    const LAST_SCANLINE_BEFORE_VBLANK: u8 = 143;
    const LAST_SCANLINE: u8 = 153;

    pub fn m_cycle(&mut self, mut bus: PpuBus<'_>) {
        let lcdc = Lcdc::from_bits_truncate(bus.io.lcdc.get());
        self.lcd_enable = lcdc.contains(Lcdc::LCD_PPU_ENABLE);

        if !self.lcd_enable {
            return;
        }

        for _ in 0..Self::DOT_CYCLES {
            self.dots += 1;

            match self.mode {
                0 => self.m0_dot(&mut bus),
                1 => self.m1_dot(&mut bus),
                2 => self.m2_dot(&mut bus),
                3 => self.m3_dot(&mut bus),
                _ => unreachable!(),
            }
        }
    }

    fn m0_dot(&mut self, bus: &mut PpuBus<'_>) {
        if self.dots >= Self::M0_DOTS {
            self.next_scanline(bus);
        }
    }

    fn m1_dot(&mut self, bus: &mut PpuBus<'_>) {
        if self.dots >= Self::SCANLINE_DOTS {
            self.next_scanline(bus);
        }
    }
    
    fn m2_dot(&mut self, bus: &mut PpuBus<'_>) {
        if self.dots >= Self::M2_DOTS {
            self.enter_mode(3, bus);
        }
    }
    
    fn m3_dot(&mut self, bus: &mut PpuBus<'_>) {
        if self.dots >= Self::M3_DOTS {
            self.enter_mode(0, bus);
        }
    }

     fn next_scanline(&mut self, bus: &mut PpuBus<'_>) {
        let ly = bus.io.ly.get();

        if ly == Self::LAST_SCANLINE_BEFORE_VBLANK {
            self.enter_mode(1, bus);
            bus.io.ly.set(ly + 1);
        } else {
            self.enter_mode(2, bus);
            let ly = if ly == Self::LAST_SCANLINE { 0 } else { ly + 1 };
            bus.io.ly.set(ly);
        }
    }

    fn enter_mode(&mut self, mode: u8, bus: &mut PpuBus<'_>) {
        self.dots = 0;
        self.mode = mode;

        let stat = bus.io.stat.get() & !0b0000_0011;
        let stat = stat | (mode & 0b0000_0011);
        bus.io.stat.set(stat);
    }
}

#[derive(Default)]
pub struct Bus {
    cart: Cartridge,
    ppu: Ppu,
    wram: Memory<0x2000, 0xC000>,
    hram: Memory<0x007F, 0xFF80>,
    io: IoRegisters,
    ier: Memory<0x0001, 0xFFFF>,
}

impl From<Cartridge> for Bus {
    fn from(cart: Cartridge) -> Self {
        Self {
            cart,
            ..Default::default()
        }
    }
}

impl Bus {
    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x7FFF => self.cart.read(addr),
            0x8000..=0x9FFF => self.ppu.read(addr),
            0xC000..=0xDFFF => self.wram.read(addr),
            0xE000..=0xFDFF => self.wram.read(addr - 0x2000),
            0xFE00..=0xFE9F => self.ppu.read(addr),
            0xFEA0..=0xFEFF => 0x00, // TODO: OAM corruption
            0xFF00..=0xFF7F => self.io.read(addr),
            0xFF80..=0xFFFE => self.hram.read(addr),
            0xFFFF => self.ier.get(),
            _ => panic!("Tried to read from address {:#06X}", addr),
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x7FFF => self.cart.write(addr, value),
            0x8000..=0x9FFF => self.ppu.write(addr, value),
            0xC000..=0xDFFF => self.wram.write(addr, value),
            0xE000..=0xFDFF => self.wram.write(addr - 0x2000, value),
            0xFE00..=0xFE9F => self.ppu.write(addr, value),
            0xFEA0..=0xFEFF => {} // unused
            0xFF80..=0xFFFE => self.hram.write(addr, value),
            0xFF00..=0xFF7F => self.io.write(addr, value),
            0xFFFF => self.ier.set(value),
            _ => panic!("Tried to write to address {:#06X}", addr),
        }
    }

    pub fn sync(&mut self) {
        self.ppu.m_cycle(PpuBus { io: &mut self.io });
    }
}

#[derive(Default)]
pub struct Cartridge {
    // TODO mappers
    rom: Memory<0x8000, 0x0000>,
    ram: Memory<0x2000, 0xA000>,
}

impl Cartridge {
    pub fn new<R: io::BufRead>(r: &mut R) -> io::Result<Self> {
        let rom = Memory::new(r)?;
        Ok(Self {
            rom,
            ..Default::default()
        })
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x7FFF => self.rom.read(addr),
            0xA000..=0xBFFF => self.ram.read(addr),
            _ => panic!("Tried to read from cartridge address {:#06X}", addr),
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x7FFF => self.rom.write(addr, value),
            0xA000..=0xBFFF => self.ram.write(addr, value),
            _ => panic!("Tried to write to cartridge address {:#06X}", addr),
        }
    }
}

bitflags! {
    #[derive(Default, Clone, Copy)]
    pub struct Stat: u8 {
        const COMPARE_INT  = 0b0100_0000;
        const MODE_2_INT   = 0b0010_0000;
        const MODE_1_INT   = 0b0001_0000;
        const MODE_0_INT   = 0b0000_1000;
        const COMPARE_FLAG = 0b0000_0100;
        const PPU_MODE_H   = 0b0000_0010;
        const PPU_MODE_L   = 0b0000_0001;
    }
}

bitflags! {
    #[derive(Default, Clone, Copy)]
    pub struct Lcdc: u8 {
        const LCD_PPU_ENABLE   = 0b1000_0000;
        const WINDOW_TILE_MAP  = 0b0100_0000;
        const WINDOW_ENABLE    = 0b0010_0000;
        const BG_WINDOW_TILES  = 0b0001_0000;
        const BG_TILE_MAP      = 0b0000_1000;
        const OBJ_SIZE         = 0b0000_0100;
        const OBJ_ENABLE       = 0b0000_0010;
        const BG_WINDOW_ENABLE = 0b0000_0001;
    }
}

#[derive(Default)]
pub struct IoRegisters {
    pub joyp: Memory<0x01, 0xFF00>,
    pub srd:  Memory<0x01, 0xFF01>,
    pub srt:  Memory<0x01, 0xFF02>,
    pub tma:  Memory<0x01, 0xFF06>,
    pub ifr:  Memory<0x01, 0xFF0F>,
    pub nr10: Memory<0x01, 0xFF10>,
    pub nr12: Memory<0x01, 0xFF12>,
    pub nr14: Memory<0x01, 0xFF14>,
    pub nr22: Memory<0x01, 0xFF17>,
    pub nr24: Memory<0x01, 0xFF19>,
    pub nr30: Memory<0x01, 0xFF1A>,
    pub nr42: Memory<0x01, 0xFF21>,
    pub nr44: Memory<0x01, 0xFF23>,
    pub nr50: Memory<0x01, 0xFF24>,
    pub nr51: Memory<0x01, 0xFF25>,
    pub nr52: Memory<0x01, 0xFF26>,
    pub lcdc: Memory<0x01, 0xFF40>,
    pub stat: Memory<0x01, 0xFF41>,
    pub scy:  Memory<0x01, 0xFF42>,
    pub scx:  Memory<0x01, 0xFF43>,
    pub ly:   Memory<0x01, 0xFF44>,
    pub lyc:  Memory<0x01, 0xFF45>,
    pub bgp:  Memory<0x01, 0xFF47>,
    pub obp0: Memory<0x01, 0xFF48>,
    pub obp1: Memory<0x01, 0xFF49>,
    pub wy:   Memory<0x01, 0xFF4A>,
    pub wx:   Memory<0x01, 0xFF4B>,
}

impl IoRegisters {
    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0xFF00 => self.joyp.get(),
            0xFF01 => self.srd.get(),
            0xFF02 => self.srt.get(),
            0xFF06 => self.tma.get(),
            0xFF0F => self.ifr.get(),
            0xFF10 => self.nr10.get(),
            0xFF12 => self.nr12.get(),
            0xFF14 => self.nr14.get(),
            0xFF17 => self.nr22.get(),
            0xFF19 => self.nr24.get(),
            0xFF1A => self.nr30.get(),
            0xFF21 => self.nr42.get(),
            0xFF23 => self.nr44.get(),
            0xFF24 => self.nr50.get(),
            0xFF25 => self.nr51.get(),
            0xFF26 => self.nr52.get(),
            0xFF40 => self.lcdc.get(),
            0xFF41 => self.stat.get(),
            0xFF42 => self.scy.get(),
            0xFF43 => self.scx.get(),
            0xFF44 => self.ly.get(),
            0xFF45 => self.lyc.get(),
            0xFF47 => self.bgp.get(),
            0xFF48 => self.obp0.get(),
            0xFF49 => self.obp1.get(),
            0xFF4A => self.wy.get(),
            0xFF4B => self.wx.get(),
            0xFF7F => 0xFF, 
            _ => panic!("Tried to read IO register at address {:#06X}", addr),
        }
    }
    
    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0xFF00 => self.joyp.set(value),
            0xFF01 => self.srd.set(value),
            0xFF02 => self.srt.set(value),
            0xFF06 => self.tma.set(value),
            0xFF0F => self.ifr.set(value),
            0xFF10 => self.nr10.set(value),
            0xFF12 => self.nr12.set(value),
            0xFF14 => self.nr14.set(value),
            0xFF17 => self.nr22.set(value),
            0xFF19 => self.nr24.set(value),
            0xFF1A => self.nr30.set(value),
            0xFF21 => self.nr42.set(value),
            0xFF24 => self.nr50.set(value),
            0xFF23 => self.nr44.set(value),
            0xFF25 => self.nr51.set(value),
            0xFF26 => self.nr52.set(value),
            0xFF40 => self.lcdc.set(value),
            0xFF41 => self.stat.set(value),
            0xFF42 => self.scy.set(value),
            0xFF43 => self.scx.set(value),
            0xFF44 => self.ly.set(value),
            0xFF45 => self.lyc.set(value),
            0xFF47 => self.bgp.set(value),
            0xFF48 => self.obp0.set(value),
            0xFF49 => self.obp1.set(value),
            0xFF4A => self.wy.set(value),
            0xFF4B => self.wx.set(value),
            0xFF7F => {}, 
            _ => panic!("Tried to write to IO register at address {:#06X} [{:#04X} {:#010b}]", addr, value, value),
        }
    }
}

pub struct Memory<const SIZE: usize, const OFFSET: u16> {
    data: [u8; SIZE],
}

impl<const SIZE: usize, const OFFSET: u16> Default for Memory<SIZE, OFFSET> {
    fn default() -> Self {
        Self { data: [0u8; SIZE] }
    }
}

impl<const SIZE: usize, const OFFSET: u16> Memory<SIZE, OFFSET> {
    pub fn new<R: io::BufRead>(r: &mut R) -> io::Result<Self> {
        let mut mem = Self::default();
        r.read_exact(&mut mem.data)?;
        Ok(mem)
    }

    pub fn read(&self, addr: u16) -> u8 {
        let mapped_addr = Self::map_addr(addr);
        self.data[mapped_addr as usize]
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        let mapped_addr = Self::map_addr(addr);
        self.data[mapped_addr as usize] = value;
    }

    fn map_addr(addr: u16) -> u16 {
        addr - OFFSET
    }
}

impl<const OFFSET: u16> Memory<1, OFFSET> {
    pub fn get(&self) -> u8 {
        self.data[0]
    }

    pub fn set(&mut self, value: u8) {
        self.data[0] = value;
    }
}

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long)]
    file: String,
}

fn main() -> io::Result<()> {
    let args = Args::parse(); 
    let file = File::open(args.file)?;
    let mut r = BufReader::new(file);

    let mut cpu = Cpu::default();
    cpu.load_cart(Cartridge::new(&mut r)?);

    loop {
        cpu.decode_execute();
    }
}
