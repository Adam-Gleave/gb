use crate::bus::Memory;

#[derive(Default)]
pub struct IoRegisters {
    pub joyp: Memory<0x01, 0xFF00>,
    pub srd:  Memory<0x01, 0xFF01>,
    pub srt:  Memory<0x01, 0xFF02>,
    pub div:  Memory<0x01, 0xFF04>,
    pub tima: Memory<0x01, 0xFF05>,
    pub tma:  Memory<0x01, 0xFF06>,
    pub tac:  Memory<0x01, 0xFF07>,
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
    pub fn new() -> Self {
        Self {
            lcdc: Memory::init(0x91),
            stat: Memory::init(0x85),
            ..Default::default()
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0xFF00 => self.joyp.get(),
            0xFF01 => self.srd.get(),
            0xFF02 => self.srt.get(),
            0xFF04 => self.div.get(),
            0xFF05 => self.tima.get(),
            0xFF06 => self.tma.get(),
            0xFF07 => self.tac.get(),
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
            // 0xFF44 => 0x90,
            0xFF45 => self.lyc.get(),
            0xFF47 => self.bgp.get(),
            0xFF48 => self.obp0.get(),
            0xFF49 => self.obp1.get(),
            0xFF4A => self.wy.get(),
            0xFF4B => self.wx.get(),
            _ => 0xFF,
            // _ => panic!("Tried to read IO register at address {:#06X}", addr),
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0xFF00 => self.joyp.set(value),
            0xFF01 => self.srd.set(value),
            0xFF02 => self.srt.set(value),
            0xFF04 => self.div.set(value),
            0xFF05 => self.tima.set(value),
            0xFF06 => self.tma.set(value),
            0xFF07 => self.tac.set(value),
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
            0xFF7F => {}
            _ => panic!(
                "Tried to write to IO register at address {:#06X} [{:#04X} {:#010b}]",
                addr, value, value
            ),
        }
    }
}
