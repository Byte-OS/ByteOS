#![no_std]
#![feature(used_with_arg)]

extern crate alloc;

use alloc::{sync::Arc, vec::Vec};
use devices::{
    device::{DeviceType, Driver, UartDriver},
    driver_define,
    fdt::Node,
    node_to_interrupts, register_device_irqs, VIRT_ADDR_START,
};
use log::info;
use ns16550a::{
    Break, DMAMode, Divisor, ParityBit, ParitySelect, StickParity, StopBits, Uart, WordLength,
};

pub struct NS16550a {
    _base: usize,
    inner: Uart,
    irqs: Vec<u32>,
}

impl Driver for NS16550a {
    fn get_id(&self) -> &str {
        "ns16550a"
    }

    fn get_device_wrapper(self: Arc<Self>) -> DeviceType {
        DeviceType::UART(self.clone())
    }

    fn try_handle_interrupt(&self, _irq: u32) -> bool {
        info!("handle uart interrupt");
        false
    }

    fn interrupts(&self) -> &[u32] {
        &self.irqs
    }
}

impl UartDriver for NS16550a {
    fn put(&self, c: u8) {
        self.inner.put(c)
    }

    fn get(&self) -> Option<u8> {
        self.inner.get()
    }
}

fn init_driver(node: &Node) -> Arc<dyn Driver> {
    let addr = node.reg().unwrap().next().unwrap().address as usize;

    info!(
        "node interrupts: {:?}",
        node.interrupts().unwrap().flatten().collect::<Vec<u32>>()
    );
    let uart = Arc::new(NS16550a {
        _base: VIRT_ADDR_START + addr,
        inner: Uart::new(VIRT_ADDR_START + addr),
        irqs: node_to_interrupts(node),
    });
    register_device_irqs(uart.clone());
    uart.inner.init(
        WordLength::EIGHT,
        StopBits::ONE,
        ParityBit::DISABLE,
        ParitySelect::EVEN,
        StickParity::DISABLE,
        Break::DISABLE,
        DMAMode::MODE0,
        Divisor::BAUD1200,
    );
    uart
}

driver_define!("ns16550a", init_driver);
