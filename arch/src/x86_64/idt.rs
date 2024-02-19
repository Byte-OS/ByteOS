use spin::once::Once;

use x86_64::structures::idt::{Entry, HandlerFunc, InterruptDescriptorTable};

const NUM_INT: usize = 256;

pub(super) static IDT: Once<IdtStruct> = Once::new();

/// A wrapper of the Interrupt Descriptor Table (IDT).
#[repr(transparent)]
pub struct IdtStruct(InterruptDescriptorTable);

impl IdtStruct {
    /// Constructs a new IDT struct that filled with entries from
    /// `trap_handler_table`.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        extern "C" {
            #[link_name = "trap_handler_table"]
            static ENTRIES: [extern "C" fn(); NUM_INT];
        }
        let mut idt = Self(InterruptDescriptorTable::new());

        let entries = unsafe {
            core::slice::from_raw_parts_mut(
                &mut idt.0 as *mut _ as *mut Entry<HandlerFunc>,
                NUM_INT,
            )
        };
        for i in 0..NUM_INT {
            entries[i].set_handler_fn(unsafe { core::mem::transmute(ENTRIES[i]) });
        }
        idt
    }

    /// Loads the IDT into the CPU (executes the `lidt` instruction).
    ///
    /// # Safety
    ///
    /// This function is unsafe because it manipulates the CPU's privileged
    /// states.
    pub unsafe fn load(&'static self) {
        self.0.load();
    }
}

pub fn init() {
    info!("Initializing IDT...");
    let idt = IdtStruct::new();
    IDT.call_once(|| idt);
    unsafe {
        IDT.get().unwrap().load();
    }
}
