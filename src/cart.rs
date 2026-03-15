use std::io;

use crate::bus::Memory;

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
