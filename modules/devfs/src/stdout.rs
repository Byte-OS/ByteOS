use logging::puts;
use vfscore::INodeInterface;

pub struct stdout;

impl INodeInterface for stdout {
    fn write(&mut self, buffer: &[u8]) -> vfscore::VfsResult<usize> {
        puts(buffer);
        Ok(buffer.len())
    }
}