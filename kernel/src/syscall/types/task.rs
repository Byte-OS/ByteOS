bitflags! {
    #[derive(Debug)]
    pub struct CloneFlags: usize {
        const CSIGNAL		        = 0x000000ff;
        const CLONE_VM	            = 0x00000100;
        const CLONE_FS	            = 0x00000200;
        const CLONE_FILES	        = 0x00000400;
        const CLONE_SIGHAND	        = 0x00000800;
        const CLONE_PIDFD	        = 0x00001000;
        const CLONE_PTRACE	        = 0x00002000;
        const CLONE_VFORK	        = 0x00004000;
        const CLONE_PARENT	        = 0x00008000;
        const CLONE_THREAD	        = 0x00010000;
        const CLONE_NEWNS	        = 0x00020000;
        const CLONE_SYSVSEM	        = 0x00040000;
        const CLONE_SETTLS	        = 0x00080000;
        const CLONE_PARENT_SETTID	= 0x00100000;
        const CLONE_CHILD_CLEARTID	= 0x00200000;
        const CLONE_DETACHED	    = 0x00400000;
        const CLONE_UNTRACED	    = 0x00800000;
        const CLONE_CHILD_SETTID	= 0x01000000;
        const CLONE_NEWCGROUP	    = 0x02000000;
        const CLONE_NEWUTS	        = 0x04000000;
        const CLONE_NEWIPC	        = 0x08000000;
        const CLONE_NEWUSER	        = 0x10000000;
        const CLONE_NEWPID	        = 0x20000000;
        const CLONE_NEWNET	        = 0x40000000;
        const CLONE_IO	            = 0x80000000;
    }
}
