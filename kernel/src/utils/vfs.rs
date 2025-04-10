use fs::VfsError;
use syscalls::Errno;

pub fn from_vfs(vfs_error: VfsError) -> Errno {
    match vfs_error {
        VfsError::NotLinkFile => Errno::EBADF,
        VfsError::NotDir => Errno::ENOTDIR,
        VfsError::NotFile => Errno::EBADF,
        VfsError::NotSupported => Errno::EPERM,
        VfsError::FileNotFound => Errno::ENOENT,
        VfsError::AlreadyExists => Errno::EEXIST,
        VfsError::InvalidData => Errno::EIO,
        VfsError::DirectoryNotEmpty => Errno::ENOTEMPTY,
        VfsError::InvalidInput => Errno::EINVAL,
        VfsError::StorageFull => Errno::EIO,
        VfsError::UnexpectedEof => Errno::EIO,
        VfsError::WriteZero => Errno::EIO,
        VfsError::Io => Errno::EIO,
        VfsError::Blocking => Errno::EAGAIN,
        VfsError::NoMountedPoint => Errno::ENOENT,
        VfsError::NotAPipe => Errno::EPIPE,
        VfsError::NotWriteable => Errno::EBADF,
    }
}
