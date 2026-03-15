use bitflags::bitflags;

use crate::Cpu;

bitflags! {
    pub struct Interrupts: u8 {
        const VBLANK = 0b0000_0001;
        const LCD    = 0b0000_0010;
        const TIMER  = 0b0000_0100;
        const SERIAL = 0b0000_1000;
        const JOYPAD = 0b0001_0000;
    }
}

impl Cpu {
    pub(super) fn handle_interrupt(&mut self) -> bool {
        if !self.ime {
            return false;
        }

        let ier = self.bus.ier.get();
        let ifr = self.bus.io.ifr.get();
        let requests = Interrupts::from_bits_truncate(ier & ifr);
        
        let mut request_handled = false;
        let mut do_isr = |flag: Interrupts, addr: u16| {
            self.ime = false;

            let ifr = self.bus.io.ifr.get();
            let ifr = ifr & !flag.bits();
            self.bus.io.ifr.set(ifr);

            self.nop();
            self.nop();
            self.do_call(addr);

            request_handled = true;
        };
        
        if requests.contains(Interrupts::VBLANK) {
            do_isr(Interrupts::VBLANK, 0x40);
        }
        if requests.contains(Interrupts::LCD) {
            do_isr(Interrupts::LCD, 0x48);
        }
        if requests.contains(Interrupts::TIMER) {
            do_isr(Interrupts::TIMER, 0x50);
        }
        if requests.contains(Interrupts::SERIAL) {
            do_isr(Interrupts::SERIAL, 0x58);
        }
        if requests.contains(Interrupts::JOYPAD) {
            do_isr(Interrupts::JOYPAD, 0x60);
        }

        request_handled
    }
}
