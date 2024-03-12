use raw_cpuid::CpuId;

static mut INIT_TICK: u64 = 0;
static mut CPU_FREQ_MHZ: u64 = 4000;

/// Converts hardware ticks to nanoseconds.
pub fn ticks_to_nanos(ticks: u64) -> u64 {
    ticks * 1_000 / unsafe { CPU_FREQ_MHZ }
}

pub(super) fn init_early() {
    info!("freq1: {:#x?}", CpuId::new().get_tsc_info());
    debug!("cpuid: {:#x?}", CpuId::new().get_vendor_info());
    if let Some(freq) = CpuId::new()
        .get_processor_frequency_info()
        .map(|info| info.processor_base_frequency())
    {
        debug!("freq: {}", freq);
        if freq > 0 {
            info!("Got TSC frequency by CPUID: {} MHz", freq);
            unsafe { CPU_FREQ_MHZ = freq as u64 }
        }
    }

    unsafe { INIT_TICK = core::arch::x86_64::_rdtsc() };
    debug!("INIT_TICK: {}", unsafe { INIT_TICK });

    unsafe {
        use x2apic::lapic::{TimerDivide, TimerMode};
        let lapic = super::apic::local_apic();
        lapic.set_timer_mode(TimerMode::Periodic);
        lapic.set_timer_divide(TimerDivide::Div256); // indeed it is Div1, the name is confusing.
        lapic.enable_timer();

        lapic.set_timer_initial(0x20_0000);
        debug!("count: {}", lapic.timer_current());
        // set_oneshot_timer(2000);
    }
}
