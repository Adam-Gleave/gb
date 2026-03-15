use bitflags::bitflags;

use crate::bus::Memory;
use crate::bus::PpuBus;

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
                println!(
                    "Tried to read from PPU-managed address {:#06X} in mode {}",
                    addr, self.mode
                );
                0xFF
            }
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x8000..=0x9FFF if !self.lcd_enable || self.mode != 3 => self.vram.write(addr, value),
            0xFE00..=0xFE9F if !self.lcd_enable || self.mode <= 1 => self.oam.write(addr, value),
            // _ => println!(
            //     // "Tried to write to PPU-managed address {:#06X} in mode {}",
            //     addr, self.mode
            // ),
            _ => {}
        }
    }

    const DOT_CYCLES: usize = 4;
    const SCANLINE_DOTS: u16 = 456;
    const M2_DOTS: u16 = 80;
    const M3_DOTS: u16 = 172;
    const M0_DOTS: u16 = 204;

    const LAST_SCANLINE_BEFORE_VBLANK: u8 = 143;
    const LAST_SCANLINE: u8 = 153;

    pub fn new() -> Self {
        Self {
            mode: 1,
            ..Default::default()
        }
    }

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
