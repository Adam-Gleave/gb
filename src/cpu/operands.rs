use crate::Cpu;
use crate::cpu::Flags;

pub trait SrcOperand8 {
    fn read(&self, cpu: &mut Cpu) -> u8;
}

pub trait DstOperand8 {
    fn write(&self, cpu: &mut Cpu, value: u8);
}

pub trait SrcOperand16 {
    fn read(&self, cpu: &mut Cpu) -> u16;
}

pub trait DstOperand16 {
    fn write(&self, cpu: &mut Cpu, value: u16);
}

#[derive(Clone, Copy, Debug)]
pub enum Register {
    A,
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
pub enum RegisterPair {
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
pub struct Imm8;

impl SrcOperand8 for Imm8 {
    fn read(&self, cpu: &mut Cpu) -> u8 {
        cpu.fetch_imm8()
    }
}

#[derive(Clone, Copy)]
pub struct Imm16;

impl SrcOperand16 for Imm16 {
    fn read(&self, cpu: &mut Cpu) -> u16 {
        cpu.fetch_imm16()
    }
}

#[derive(Clone, Copy)]
pub struct Ind8;

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
pub struct Addr16;

impl SrcOperand8 for Addr16 {
    fn read(&self, cpu: &mut Cpu) -> u8 {
        let addr = cpu.fetch_imm16();
        cpu.read_cycle(addr)
    }
}

impl DstOperand8 for Addr16 {
    fn write(&self, cpu: &mut Cpu, value: u8) {
        let addr = cpu.fetch_imm16();
        cpu.write_cycle(addr, value);
    }
}

impl DstOperand16 for Addr16 {
    fn write(&self, cpu: &mut Cpu, value: u16) {
        let addr = cpu.fetch_imm16();
        let [lo, hi] = value.to_le_bytes();
        cpu.write_cycle(addr, lo);
        let addr = addr.wrapping_add(1);
        cpu.write_cycle(addr, hi);
    }
}

#[derive(Clone, Copy)]
pub enum RegisterPtr {
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
            Self::C => (0xFF, cpu.c),
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
            Self::C => (0xFF, cpu.c),
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

#[derive(Clone, Copy)]
pub enum Cond {
    Set(Flags),
    Clear(Flags),
}

impl Cond {
    pub fn eval(&self, cpu: &Cpu) -> bool {
        match self {
            Cond::Set(flags) => cpu.f.contains(*flags),
            Cond::Clear(flags) => !cpu.f.contains(*flags),
        }
    }
}
