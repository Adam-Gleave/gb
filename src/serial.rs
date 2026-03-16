use crate::bus::SerialBus;
use crate::cpu::Interrupts;

use std::io::Write;

#[derive(Clone, Copy)]
pub enum Serial {
    Idle { srt_memory: u8 },
    TransferInProgress { cycles: u8 },
}

impl Default for Serial {
    fn default() -> Self {
        Serial::Idle { srt_memory: 0x00 }
    }
}

impl Serial {
    pub fn m_cycle(&mut self, bus: SerialBus<'_>) {
        *self = match *self {
            Self::Idle { srt_memory } => self.idle_cycle(srt_memory, bus),
            Self::TransferInProgress { cycles } => self.transfer_cycle(cycles, bus),
        };
    }

    fn idle_cycle(self, srt_memory: u8, bus: SerialBus<'_>) -> Self {
        let srt = bus.io.srt.get();
        if srt_memory & 0x80 == 0x00 && srt & 0x80 != 0x00 {
            self.print(&bus);
            self.transfer_cycle(0, bus)
        } else {
            Serial::Idle { srt_memory: srt }
        }
    }

    const TRANSFER_CYCLES: u8 = 8;

    fn transfer_cycle(self, cycles: u8, bus: SerialBus<'_>) -> Self {
        let srd = bus.io.srd.get();
        bus.io.srd.set(srd << 1);

        let cycles = cycles + 1;
        if cycles > Serial::TRANSFER_CYCLES {
            let ifr = bus.io.ifr.get() | Interrupts::SERIAL.bits();
            bus.io.ifr.set(ifr);

            let srt = bus.io.srt.get() & !0x80;
            bus.io.srt.set(srt);

            Serial::Idle { srt_memory: srt }
        } else {
            Serial::TransferInProgress { cycles }
        }
    }

    fn print(&self, bus: &SerialBus<'_>) {
        let c = bus.io.srd.get().to_ascii_uppercase();
        print!("{}", c as char);
        std::io::stdout().flush().unwrap();
    }
}
