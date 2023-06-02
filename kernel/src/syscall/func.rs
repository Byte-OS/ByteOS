use fs::TimeSpec;

use super::time::current_nsec;

pub fn timespc_now() -> TimeSpec {
    let ns = current_nsec();

    TimeSpec {
        sec: ns / 1_000_000_000,
        nsec: (ns % 1_000_000_000) / 1000,
    }
}
