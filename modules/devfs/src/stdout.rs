use logging::puts;
use vfscore::INodeInterface;

pub struct Stdout;

impl INodeInterface for Stdout {
    fn write(&self, buffer: &[u8]) -> vfscore::VfsResult<usize> {
        puts(buffer);
        Ok(buffer.len())
    }
}
