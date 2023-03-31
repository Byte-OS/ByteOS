#![no_std]

#[macro_use]
extern crate log;
#[macro_use]
extern crate alloc;

pub mod memory;

use fdt::{self, Fdt};
use core::sync::atomic::{AtomicUsize, Ordering};

pub static DEVICE_TREE_ADDR: AtomicUsize = AtomicUsize::new(0);

pub fn init_device(device_tree: usize) {
    DEVICE_TREE_ADDR.store(device_tree, Ordering::Relaxed);
    let fdt = unsafe { Fdt::from_ptr(DEVICE_TREE_ADDR.load(Ordering::Relaxed) as *const u8).unwrap() };
    info!("This is a devicetree representation of a {}", fdt.root().model());
    info!("...which is compatible with at least: {}", fdt.root().compatible().first());
    info!("...and has {} CPU(s)", fdt.cpus().count());
    
    fdt.memory().regions().for_each(|x| {
        info!("memory region {:#X} - {:#X}", 
            x.starting_address as usize,
            x.starting_address as usize + x.size.unwrap()
        );
    });

    let chosen = fdt.chosen();
    if let Some(bootargs) = chosen.bootargs() {
        info!("The bootargs are: {:?}", bootargs);
    }

    if let Some(stdout) = chosen.stdout() {
        info!("It would write stdout to: {}", stdout.name);
    }

    let node = fdt.all_nodes();

    for child in node {
        if let Some(compatible) = child.compatible() {
            info!("    {}  {}", child.name, compatible.first());
        }
    }

    // let soc = fdt.find_node("/soc");
    // info!("Does it have a `/soc` node? {}", if soc.is_some() { "yes" } else { "no" });
    // if let Some(soc) = soc {
    //     info!("...and it has the following children:");
    //     for child in soc.children() {
    //         info!("    {}", child.name);
    //         if let Some(child) = child.compatible() {
    //             info!("{}", child.first());
    //         }
    //     }
    // }
}
