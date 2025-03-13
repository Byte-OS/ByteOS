/*
base + 0x000000: Reserved (interrupt source 0 does not exist)
base + 0x000004: Interrupt source 1 priority
base + 0x000008: Interrupt source 2 priority
...
base + 0x000FFC: Interrupt source 1023 priority
base + 0x001000: Interrupt Pending bit 0-31
base + 0x00107C: Interrupt Pending bit 992-1023
...
base + 0x002000: Enable bits for sources 0-31 on context 0
base + 0x002004: Enable bits for sources 32-63 on context 0
...
base + 0x00207C: Enable bits for sources 992-1023 on context 0
base + 0x002080: Enable bits for sources 0-31 on context 1
base + 0x002084: Enable bits for sources 32-63 on context 1
...
base + 0x0020FC: Enable bits for sources 992-1023 on context 1
base + 0x002100: Enable bits for sources 0-31 on context 2
base + 0x002104: Enable bits for sources 32-63 on context 2
...
base + 0x00217C: Enable bits for sources 992-1023 on context 2
...
base + 0x1F1F80: Enable bits for sources 0-31 on context 15871
base + 0x1F1F84: Enable bits for sources 32-63 on context 15871
base + 0x1F1FFC: Enable bits for sources 992-1023 on context 15871
...
base + 0x1FFFFC: Reserved
base + 0x200000: Priority threshold for context 0
base + 0x200004: Claim/complete for context 0
base + 0x200008: Reserved
...
base + 0x200FFC: Reserved
base + 0x201000: Priority threshold for context 1
base + 0x201004: Claim/complete for context 1
...
base + 0x3FFF000: Priority threshold for context 15871
base + 0x3FFF004: Claim/complete for context 15871
base + 0x3FFF008: Reserved
...
base + 0x3FFFFFC: Reserved
*/

use core::ptr::{read_volatile, write_volatile};

use crate::PLIC;

impl PLIC {
    // enable a interrupt.
    pub fn set_irq_enable(&self, hart_id: u32, is_smode: bool, irq: u32) {
        // hart 0 M-Mode, addr: base + 0x2000, a irq in per bit.
        // such as enable irq 173 is base + 0x2000 + 173 /32, bit 173 % 32.
        unsafe {
            let mut addr = self.base + 0x2000 + irq as usize / 32;
            // if is_smode {
            //     addr += 0x80;
            // }
            addr += (hart_id as usize * 2 + is_smode as usize) * 0x80;
            write_volatile(
                addr as *mut u32,
                read_volatile(addr as *const u32) | (1 << irq),
            )
        }
    }

    pub fn get_irq_claim(&self, hart_id: u32, is_smode: bool) -> u32 {
        // return irq claim
        // addr: base + 0x20_0000
        let mut addr = self.base + 0x20_0004;
        addr += (hart_id as usize * 2 + is_smode as usize) * 0x1000;
        unsafe { read_volatile(addr as *const u32) }
    }

    pub fn complete_irq_claim(&self, hart_id: u32, is_smode: bool, irq: u32) {
        let mut addr = self.base + 0x20_0004;
        addr += (hart_id as usize * 2 + is_smode as usize) * 0x1000;
        unsafe {
            write_volatile(addr as *mut u32, irq);
        }
    }

    pub fn set_thresold(&self, hart_id: u32, is_smode: bool, thresold: u32) {
        let mut addr = self.base + 0x20_0000;
        addr += (hart_id as usize * 2 + is_smode as usize) * 0x1000;
        unsafe {
            write_volatile(addr as *mut u32, thresold);
        }
    }

    pub fn set_priority(&self, irq: u32, priority: u32) {
        // base + 4 x interruptID, value range: 0 - 7
        unsafe {
            // set priority to 7
            write_volatile((self.base + (irq as usize) * 4) as *mut u32, priority);
        }
    }
}
