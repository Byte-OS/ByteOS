use alloc::sync::Arc;
use srv_iface::UART_IMPLS;

use crate::{
    device::{Driver, UartDriver},
    driver_define,
};

pub struct IPCUart;

impl Driver for IPCUart {
    fn get_id(&self) -> &str {
        "ipc-uart"
    }

    fn get_device_wrapper(self: alloc::sync::Arc<Self>) -> crate::device::DeviceType {
        crate::device::DeviceType::UART(self)
    }
}

impl UartDriver for IPCUart {
    fn put(&self, c: u8) {
        UART_IMPLS[0].lock().putchar(c);
    }

    fn puts(&self, bytes: &[u8]) {
        UART_IMPLS[0].lock().puts(bytes);
    }

    fn get(&self) -> Option<u8> {
        Some(UART_IMPLS[0].lock().getchar())
    }
}
