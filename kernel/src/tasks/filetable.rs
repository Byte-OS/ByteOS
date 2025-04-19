use alloc::{sync::Arc, vec::Vec};
use core::ops::{Deref, DerefMut};
use fs::file::File;
use vfscore::OpenFlags;

const FILE_MAX: usize = 255;
const FD_NONE: Option<Arc<File>> = Option::None;

#[derive(Clone)]
pub struct FileTable(pub Vec<Option<Arc<File>>>);

impl FileTable {
    pub fn new() -> Self {
        let mut file_table: Vec<Option<Arc<File>>> = vec![FD_NONE; FILE_MAX];
        file_table[..3].fill(Some(
            File::open("/dev/ttyv0".into(), OpenFlags::O_RDWR)
                .map(Arc::new)
                .expect("can't read tty file"),
        ));
        Self(file_table)
    }
}

impl Deref for FileTable {
    type Target = Vec<Option<Arc<File>>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for FileTable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub fn rlimits_new() -> Vec<usize> {
    let mut rlimits = vec![0usize; 8];
    rlimits[7] = FILE_MAX;
    rlimits
}
