mod dma;
mod io;
mod memory;

use self::dma::Dma;
pub use self::io::IoRegisters;
pub use self::memory::Memory;

use crate::Cartridge;
use crate::ppu::Ppu;
use crate::serial::Serial;
use crate::timer::Timer;

#[derive(Default)]
pub struct Bus {
    pub cart: Cartridge,
    pub ppu: Ppu,
    pub wram: Memory<0x2000, 0xC000>,
    pub hram: Memory<0x007F, 0xFF80>,
    pub io: IoRegisters,
    pub ier: Memory<0x0001, 0xFFFF>,
    pub timer: Timer,
    pub serial: Serial,
    pub dma: Dma,
}

impl From<Cartridge> for Bus {
    fn from(cart: Cartridge) -> Self {
        Self {
            cart,
            ppu: Ppu::new(),
            io: IoRegisters::new(),
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
            // _ => println!("Tried to write to address {:#06X} [{:#06X}]", addr, value),
        }
    }

    pub fn sync(&mut self) {
        self.timer.m_cycle(TimerBus { io: &mut self.io });
        self.dma_m_cycle();
        self.serial.m_cycle(SerialBus { io: &mut self.io });
        self.ppu.m_cycle(PpuBus { io: &mut self.io });
    }
}

pub struct PpuBus<'a> {
    pub io: &'a mut IoRegisters,
}

pub struct TimerBus<'a> {
    pub io: &'a mut IoRegisters,
}

pub struct SerialBus<'a> {
    pub io: &'a mut IoRegisters,
}
