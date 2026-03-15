use crate::bus::TimerBus;
use crate::cpu::Interrupts;

#[derive(Default)]
pub struct Timer {
    div_memory: u8,
    div_cycles: u8,
    tima_cycles: u16,
    tima_memory: u8,
}

impl Timer {
    const DIV_INCREMENT: u8 = 64;

    pub fn m_cycle(&mut self, bus: TimerBus<'_>) {
        let div = bus.io.div.get();

        if div != self.div_memory {
            bus.io.div.set(0x00);
        } else if self.div_cycles >= Self::DIV_INCREMENT {
            bus.io.div.set(div.wrapping_add(1));
        }

        self.div_memory = bus.io.div.get();
        self.div_cycles = self.div_cycles.wrapping_add(1);

        let tac = bus.io.tac.get();
        let enable = tac & 0b0000_0100;
        if enable == 0x00 {
            return;
        }

        let frequency = tac & 0b0000_0011;
        let tima_increment = match frequency {
            0b00 => 256,
            0b01 => 4,
            0b10 => 16,
            0b11 => 64,
            _ => unreachable!(),
        };

        if self.tima_cycles >= tima_increment {
            self.tima_cycles = 0;

            let tima = bus.io.tima.get();
            let count = tima.wrapping_add(1);
            let modulo = bus.io.tma.get();

            let mut tima_set = false;
            if count == 0 {
                bus.io.tima.set(modulo);
                
                let ifr = bus.io.ifr.get() | Interrupts::TIMER.bits();
                bus.io.ifr.set(ifr);

                if tima != self.tima_memory {
                   bus.io.tima.set(self.tima_memory);
                   tima_set = true;
                }
            }

            if !tima_set {
                bus.io.tima.set(count);
            }
        } else {
            self.tima_cycles = self.tima_cycles.wrapping_add(1);
        }

        self.tima_memory = bus.io.tima.get();
    }
}
