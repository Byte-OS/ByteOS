use signal::{SigProcMask, SignalFlags};

#[derive(Debug, Clone)]
pub struct SignalList {
    pub signal: usize,
}

impl SignalList {
    pub fn new() -> Self {
        Self { signal: 0 }
    }

    pub fn add_signal(&mut self, signal: SignalFlags) {
        self.signal |= signal.bits() as usize;
    }

    pub fn has_signal(&self) -> bool {
        self.signal != 0
    }

    pub fn handle_signal(&mut self) -> Option<SignalFlags> {
        for i in 0..64 {
            if self.signal & (1 << i) != 0 {
                self.signal &= !(1 << i);
                return Some(SignalFlags::from_bits_truncate(1 << i));
            }
        }
        None
    }

    pub fn try_get_signal(&self) -> Option<SignalFlags> {
        for i in 0..64 {
            if self.signal & (1 << i) != 0 {
                return Some(SignalFlags::from_bits_truncate(1 << i));
            }
        }
        None
    }

    pub fn remove_signal(&mut self, signal: SignalFlags) {
        self.signal &= !signal.bits() as usize;
    }

    pub fn has_sig(&self, signal: SignalFlags) -> bool {
        // self.signal & !signal.bits() as usize != 0
        self.signal & signal.bits() as usize != 0
    }

    pub fn mask(&self, mask: SigProcMask) -> SignalList {
        SignalList {
            signal: !mask.mask & self.signal,
        }
    }
}
