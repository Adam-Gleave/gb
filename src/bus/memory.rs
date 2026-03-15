use std::io;

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

    pub fn init(value: u8) -> Self {
        Self { data: [value] }
    }
}
