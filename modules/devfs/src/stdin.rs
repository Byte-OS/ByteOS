use arch::console_getchar;
use log::info;
use vfscore::INodeInterface;

pub struct Stdin;

impl INodeInterface for Stdin {
    fn read(&self, buffer: &mut [u8]) -> vfscore::VfsResult<usize> {
        info!("buffer len: {}", buffer.len());
        assert!(buffer.len() > 0);
        let mut c = console_getchar() as i8;
        loop {
            if c != -1 {
                break;
            }
            c = console_getchar() as i8;
        }
        buffer[0] = c as u8;
        Ok(1)
    }
}