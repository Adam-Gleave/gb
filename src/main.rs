use std::fs::File;
use std::io;
use std::io::BufReader;

use bitflags::bitflags;
use clap::Parser;

bitflags! {
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
            0xC3 => self.jp(),
            _ => panic!("Unexpected opcode {:#02X}", self.opcode),
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

    fn write_cycle(&mut self, addr: u16, value: u8) {
        self.cycles = self.cycles.wrapping_add(1);
        self.bus.write(addr, value);
    }

    fn noop(&mut self) {
        self.prefetch(self.pc.get());
    }

    fn jp(&mut self) {
        let addr = self.fetch_imm16();
        self.do_jp(addr);
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

    fn do_jp(&mut self, addr: u16) {
        self.pc.set(addr);
        self.cycle();
        self.prefetch(self.pc.get());
    }
}

#[derive(Default)]
pub struct Bus {
    cart: Cartridge,
    vram: Memory<0x2000, 0x8000>,
    wram: Memory<0x2000, 0xC000>,
    hram: Memory<0x007E, 0xFF80>,
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
            0xFF80..=0xFFFE => self.hram.read(addr),
            0xFFFF => self.ier.read(addr),
            _ => panic!(),
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x7FFF => self.cart.write(addr, value),
            0x8000..=0x9FFF => self.vram.write(addr, value),
            0xC000..=0xDFFF => self.wram.write(addr, value),
            0xE000..=0xFDFF => self.wram.write(addr - 0x2000, value),
            0xFF80..=0xFFFE => self.hram.write(addr, value),
            0xFFFF => self.ier.write(addr, value),
            _ => panic!(),
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
            _ => panic!(),
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x7FFF => self.rom.write(addr, value),
            0xA000..=0xBFFF => self.ram.write(addr, value),
            _ => panic!(),
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
