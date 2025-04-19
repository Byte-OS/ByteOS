use polyhal::MappingFlags;

bitflags! {
    // MAP Flags
    #[derive(Debug)]
    pub struct MapFlags: u32 {
        const MAP_SHARED          =    0x01;
        const MAP_PRIVATE         =    0x02;
        const MAP_SHARED_VALIDATE =    0x03;
        const MAP_TYPE            =    0x0f;
        const MAP_FIXED           =    0x10;
        const MAP_ANONYMOUS       =    0x20;
        const MAP_NORESERVE       =    0x4000;
        const MAP_GROWSDOWN       =    0x0100;
        const MAP_DENYWRITE       =    0x0800;
        const MAP_EXECUTABLE      =    0x1000;
        const MAP_LOCKED          =    0x2000;
        const MAP_POPULATE        =    0x8000;
        const MAP_NONBLOCK        =    0x10000;
        const MAP_STACK           =    0x20000;
        const MAP_HUGETLB         =    0x40000;
        const MAP_SYNC            =    0x80000;
        const MAP_FIXED_NOREPLACE =    0x100000;
        const MAP_FILE            =    0;
    }

    #[derive(Debug, Clone, Copy)]
    pub struct MmapProt: u32 {
        const PROT_READ = 1 << 0;
        const PROT_WRITE = 1 << 1;
        const PROT_EXEC = 1 << 2;
    }

    #[derive(Debug)]
    pub struct MSyncFlags: u32 {
        const ASYNC = 1 << 0;
        const INVALIDATE = 1 << 1;
        const SYNC = 1 << 2;
    }

    #[derive(Debug)]
    pub struct ProtFlags: u32 {
        const PROT_NONE = 0;
        const PROT_READ = 1;
        const PROT_WRITE = 2;
        const PROT_EXEC = 4;
    }

}

impl Into<MappingFlags> for MmapProt {
    fn into(self) -> MappingFlags {
        let mut res = MappingFlags::empty();
        if self.contains(Self::PROT_READ) {
            res |= MappingFlags::R;
        }
        if self.contains(Self::PROT_WRITE) {
            res |= MappingFlags::W;
        }
        if self.contains(Self::PROT_EXEC) {
            res |= MappingFlags::X;
        }
        res
    }
}
