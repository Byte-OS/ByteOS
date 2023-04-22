#![no_std]

pub trait SignalOps {
    fn add_signal(&self, signum: usize);
    fn handle_signal(&self) -> usize;
    fn check_signal(&self) -> usize;
    fn sigmask(&self) -> usize;
}
