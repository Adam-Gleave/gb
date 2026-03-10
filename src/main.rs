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

#[derive(Clone, Copy)]
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

#[derive(Clone, Copy)]
enum RegisterPair {
    AF,
    BC,
    DE,
    HL,
}

impl SrcOperand16 for RegisterPair {
    fn read(&self, cpu: &mut Cpu) -> u16 {
        match self {
            Self::AF => Cpu::load_r16(cpu.a, cpu.f.bits()),
            Self::BC => Cpu::load_r16(cpu.b, cpu.c),
            Self::DE => Cpu::load_r16(cpu.d, cpu.e),
            Self::HL => Cpu::load_r16(cpu.h, cpu.l),
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

impl DstOperand8 for Imm8 {
    fn write(&self, cpu: &mut Cpu, value: u8) {
        let addr_offset = cpu.fetch_imm8();
        cpu.write_cycle_hi(addr_offset, value);
    }
}

#[derive(Clone, Copy)]
struct Imm16;

impl SrcOperand16 for Imm16 {
    fn read(&self, cpu: &mut Cpu) -> u16 {
        cpu.fetch_imm16()
    }
}

impl DstOperand16 for Imm16 {
    fn write(&self, cpu: &mut Cpu, value: u16) {
        let addr = cpu.fetch_imm16();
        let [value_lo, value_hi] = value.to_le_bytes();
        cpu.write_cycle(addr, value_lo);
        let addr = addr.wrapping_add(1);
        cpu.write_cycle(addr, value_hi);
    }
}

#[derive(Clone, Copy)]
enum RegisterPtr {
    HL,
    HLD,
    HLI,
}

impl SrcOperand8 for RegisterPtr {
    fn read(&self, cpu: &mut Cpu) -> u8 {
        let addr = Cpu::load_r16(cpu.h, cpu.l);
        let value = cpu.read_cycle(addr);        
        let addr = match self {
            Self::HL => addr,
            Self::HLD => addr.wrapping_sub(addr),
            Self::HLI => addr.wrapping_add(addr),
        };
        Cpu::store_r16(&mut cpu.h, &mut cpu.l, addr);
        value
    }
}

impl DstOperand8 for RegisterPtr {
    fn write(&self, cpu: &mut Cpu, value: u8) {
        let addr = Cpu::load_r16(cpu.h, cpu.l);
        cpu.write_cycle(addr, value);
        let addr = match self {
            Self::HL => addr,
            Self::HLD => addr.wrapping_sub(addr),
            Self::HLI => addr.wrapping_add(addr),
        };
        Cpu::store_r16(&mut cpu.h, &mut cpu.l, addr);
    }
}

bitflags! {
    #[derive(Default, Clone, Copy)]
    pub struct Flags: u8 {
        const Z = 0b1000_000;
        const N = 0b0100_000;
        const H = 0b0010_000;
        const C = 0b0001_000;
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
        match self.opcode {
            0x00 => self.noop(),
            0x05 => self.dec_b(),
            0x06 => self.load_8_8(Register::B, Imm8),
            0x0D => self.dec_c(),
            0x0E => self.load_8_8(Register::C, Imm8),
            0x20 => self.jr_nz(),
            0x21 => self.load_16_16(RegisterPair::HL, Imm16),
            0x32 => self.load_8_8(RegisterPtr::HLD, Register::A),
            0x3E => self.load_8_8(Register::A, Imm8),
            0xA8 => self.xor(Register::B),
            0xA9 => self.xor(Register::C),
            0xAA => self.xor(Register::D),
            0xAB => self.xor(Register::E),
            0xAC => self.xor(Register::H),
            0xAD => self.xor(Register::L),
            // 0xAE
            0xAF => self.xor(Register::A),
            0xC3 => self.jp(),
            0xE0 => self.load_8_8(Imm8, Register::A),
            0xF3 => self.di(),
            _ => panic!("Unexpected opcode {:#04X}", self.opcode),
        }
    }

    fn prefetch(&mut self, addr: u16) {
        self.opcode = self.read_cycle(addr);
        self.pc.inc();
    }

    fn cycle(&mut self) {
        self.cycles = self.cycles.wrapping_add(1);
    }

    fn read_cycle(&mut self, addr: u16) -> u8 {
        self.cycles = self.cycles.wrapping_add(1);
        self.bus.read(addr)
    }

    fn read_cycle_hi(&mut self, addr: u8) -> u8 {
        let addr_hi = 0xFF00 | addr as u16;
        self.read_cycle(addr_hi)
    }

    fn write_cycle(&mut self, addr: u16, value: u8) {
        self.cycles = self.cycles.wrapping_add(1);
        self.bus.write(addr, value);
    }

    fn write_cycle_hi(&mut self, addr: u8, value: u8) {
        let addr_hi = 0xFF00 | addr as u16;
        self.write_cycle(addr_hi, value);
    }

    fn noop(&mut self) {
        self.prefetch(self.pc.get());
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

    fn dec_b(&mut self) {
        let value = self.b.wrapping_sub(1);
        self.b = value;
        self.try_set_z(value);
        self.f.set(Flags::N, true);
        self.f.set(Flags::H, value & 0xF == 0);
        self.prefetch(self.pc.get());
    }

    fn dec_c(&mut self) {
        let value = self.c.wrapping_sub(1);
        self.c = value;
        self.try_set_z(value);
        self.f.set(Flags::N, true);
        self.f.set(Flags::H, value & 0xF == 0);
        self.prefetch(self.pc.get());
    }

    fn jr_nz(&mut self) {
        let offset = self.fetch_imm8();
        if self.f.contains(Flags::Z) {
            self.do_jr(offset);
        }
        self.prefetch(self.pc.get());
    }

    fn xor<O: SrcOperand8>(&mut self, operand: O) {
        let operand = operand.read(self);
        let value = self.a ^ operand;
        self.a = value;
        self.try_set_z(value);
        self.prefetch(self.pc.get());
    }

    fn jp(&mut self) {
        let addr = self.fetch_imm16();
        self.do_jp(addr);
        self.prefetch(self.pc.get());
    }

    fn di(&mut self) {
        self.ime = false;
        self.prefetch(self.pc.get());
    }

    fn fetch_imm16(&mut self) -> u16 {
        let lo = self.fetch_imm8();
        let hi = self.fetch_imm8();
        u16::from_le_bytes([lo, hi])
    }

    fn fetch_imm8(&mut self) -> u8 {
        let value = self.read_cycle(self.pc.get());
        self.pc.inc();
        value
    }

    fn do_jr(&mut self, offset: u8) {
        let addr = self.pc.get().wrapping_add(offset as u16);
        self.pc.set(addr);
        self.cycle();
    }

    fn do_jp(&mut self, addr: u16) {
        self.pc.set(addr);
        self.cycle();
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

#[derive(Default)]
pub struct Bus {
    cart: Cartridge,
    vram: Memory<0x2000, 0x8000>,
    wram: Memory<0x2000, 0xC000>,
    hram: Memory<0x007E, 0xFF80>,
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
            0x8000..=0x9FFF => self.vram.read(addr),
            0xC000..=0xDFFF => self.wram.read(addr),
            0xE000..=0xFDFF => self.wram.read(addr - 0x2000),
            0xFF00..=0xFF7F => self.io.read(addr),
            0xFF80..=0xFFFE => self.hram.read(addr),
            0xFFFF => self.ier.read(addr),
            _ => panic!("Tried to read from address {:#06X}", addr),
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x7FFF => self.cart.write(addr, value),
            0x8000..=0x9FFF => self.vram.write(addr, value),
            0xC000..=0xDFFF => self.wram.write(addr, value),
            0xE000..=0xFDFF => self.wram.write(addr - 0x2000, value),
            0xFF80..=0xFFFE => self.hram.write(addr, value),
            0xFF00..=0xFF7F => self.io.write(addr, value),
            0xFFFF => self.ier.write(addr, value),
            _ => panic!("Tried to write to address {:#046}", addr),
        }
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

#[derive(Default)]
pub struct IoRegisters {
    ifr: Memory<0x01, 0xFF0F>,
}

impl IoRegisters {
    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0xFF0F => self.ifr.read(addr),
            _ => panic!("Tried to read IO register at address {:#046}", addr),
        }
    }
    
    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0xFF0F => self.ifr.write(addr, value),
            _ => panic!("Tried to write to IO register at address {:#046}", addr),
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
