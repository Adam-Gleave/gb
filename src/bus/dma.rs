use crate::bus::Bus;

#[derive(Clone, Copy)]
pub enum Dma {
    Idle { dma_memory: u8 },
    TransferInProgress { cycles: u8, source: u8 },
}

impl Default for Dma {
    fn default() -> Self {
        Dma::Idle { dma_memory: 0x00 }
    }
}

impl Dma {
    const TRANSFER_CYCLES: u8 = 160;
}

impl Bus {
    pub(super) fn dma_m_cycle(&mut self) {
        self.dma = match self.dma {
            Dma::Idle { dma_memory } => self.dma_idle_cycle(dma_memory),
            Dma::TransferInProgress { cycles, source } => self.dma_transfer_cycle(cycles, source),
        };
    }

    fn dma_idle_cycle(&mut self, dma_memory: u8) -> Dma {
        let dma = self.io.dma.get();
        if dma != dma_memory {
            self.dma_transfer_cycle(0, dma)
        } else {
            Dma::Idle { dma_memory: dma }
        }
    }

    fn dma_transfer_cycle(&mut self, cycles: u8, source: u8) -> Dma {
        let source_mapped = ((source as u16) << 8) | cycles as u16;
        let dest_mapped = 0xFE00 as u16 | cycles as u16;

        let byte = self.read(source_mapped);
        self.write(dest_mapped, byte);

        let cycles = cycles + 1;
        if cycles >= Dma::TRANSFER_CYCLES {
            Dma::Idle {
                dma_memory: self.io.dma.get(),
            }
        } else {
            Dma::TransferInProgress { cycles, source }
        }
    }
}
