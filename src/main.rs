use std::fs::File;
use std::io;
use std::io::BufReader;

use bitflags::bitflags;
use clap::Parser;

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
            0x06 => self.ld_b_imm8(),
            0x0D => self.dec_c(),
            0x0E => self.ld_c_imm8(),
            0x20 => self.jr_nz(),
            0x21 => self.ld_hl_imm16(),
            0x32 => self.ld_hld_a(),
            0x3E => self.ld_a_imm8(),
            0xAF => self.xor_a_a(),
            0xC3 => self.jp(),
            0xE0 => self.ldh_a8_a(),
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

    fn dec_b(&mut self) {
        let value = self.b.wrapping_sub(1);
        self.b = value;
        self.try_set_z(value);
        self.f.set(Flags::N, true);
        self.f.set(Flags::H, value & 0xF == 0);
        self.prefetch(self.pc.get());
    }

    fn ld_b_imm8(&mut self) {
        let value = self.fetch_imm8();
        self.b = value;
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

    fn ld_c_imm8(&mut self) {
        let value = self.fetch_imm8();
        self.c = value;
        self.prefetch(self.pc.get());
    }

    fn jr_nz(&mut self) {
        let offset = self.fetch_imm8();
        if self.f.contains(Flags::Z) {
            self.do_jr(offset);
        }
        self.prefetch(self.pc.get());
    }

    fn ld_hl_imm16(&mut self) {
        let value = self.fetch_imm16();
        Self::store_r16(&mut self.h, &mut self.l, value);
        self.prefetch(self.pc.get());
    }

    fn ld_hld_a(&mut self) {
        let addr = Self::load_r16(self.h, self.l);
        let value = self.bus.read(addr);
        self.a = value;
        self.prefetch(self.pc.get());
    }

    fn ld_a_imm8(&mut self) {
        let value = self.fetch_imm8();
        self.a = value;
        self.prefetch(self.pc.get());
    }

    fn xor_a_a(&mut self) {
        self.a ^= self.a;
        self.prefetch(self.pc.get());
    }

    fn jp(&mut self) {
        let addr = self.fetch_imm16();
        self.do_jp(addr);
        self.prefetch(self.pc.get());
    }

    fn ldh_a8_a(&mut self) {
        let addr = self.fetch_imm8();
        self.write_cycle_hi(addr, self.a);
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
