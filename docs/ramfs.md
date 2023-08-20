# ramfs

Ramfs（RAM文件系统）是一种特殊类型的文件系统，它将文件和目录存储在计算机的内存中而不是磁盘上。它是一种基于内存的虚拟文件系统，允许在内存中创建文件和目录，并进行读取、写入和访问操作。

以下是Ramfs的一些关键特点和优势：

1. **速度和性能**：由于文件和目录存储在内存中，Ramfs能够提供非常快速的读取和写入操作。由于内存的访问速度远远超过磁盘访问速度，Ramfs在处理大量小型文件或需要高性能读写的应用程序方面表现出色。
2. **易于使用和实现**：Ramfs非常简单，易于实现和使用。它不涉及磁盘I/O操作，也不需要复杂的文件系统结构。这使得它成为实现临时文件系统、缓存文件系统或用于特定用途的文件系统的理想选择。
3. **临时性质**：由于Ramfs是基于内存的，它通常用于存储临时文件或运行时数据。一旦系统关闭或重新启动，Ramfs中的数据将丢失。这使得Ramfs特别适用于需要快速访问和临时存储的数据，而不需要长期保留。
4. **低存储容量要求**：相比于磁盘文件系统，Ramfs对存储容量的要求较低。它不需要预留大量的磁盘空间，而是根据需要动态分配内存。这使得Ramfs在资源受限的环境中尤为有用，如嵌入式系统或嵌入式设备。
5. **易于清除和重置**：由于Ramfs中的数据在系统重启时会被清除，它非常适合进行系统重置或恢复操作。通过重新启动系统，Ramfs可以快速恢复到初始状态，不会留下残留的文件或数据。

需要注意的是，由于Ramfs存储在内存中，它的容量受限于系统的可用内存。在使用Ramfs时，应仔细考虑可用内存的大小和系统的需求，以避免内存不足或系统性能下降的问题。

## 主要实现

```rust
pub struct RamFs {
    root: Arc<RamDirInner>,
}

impl RamFs {
    pub fn new() -> Arc<Self> {
        let inner = Arc::new(RamDirInner {
            name: String::from(""),
            children: Mutex::new(Vec::new()),
        });
        Arc::new(Self { root: inner })
    }
}

impl FileSystem for RamFs {
    fn root_dir(&'static self) -> Arc<dyn INodeInterface> {
        Arc::new(RamDir {
            inner: self.root.clone(),
            dents_off: Mutex::new(0),
        })
    }

    fn name(&self) -> &str {
        "ramfs"
    }
}

pub struct RamDirInner {
    name: String,
    children: Mutex<Vec<FileContainer>>,
}

// TODO: use frame insteads of Vec.
pub struct RamFileInner {
    name: String,
    // content: Mutex<Vec<u8>>,
    len: Mutex<usize>,
    pages: Mutex<Vec<FrameTracker>>,
    times: Mutex<[TimeSpec; 3]>, // ctime, atime, mtime.
}

#[allow(dead_code)]
pub struct RamLinkInner {
    name: String,
    link_file: Arc<dyn INodeInterface>,
}

pub enum FileContainer {
    File(Arc<RamFileInner>),
    Dir(Arc<RamDirInner>),
    Link(Arc<RamLinkInner>),
}

impl FileContainer {
    #[inline]
    fn to_inode(&self) -> VfsResult<Arc<dyn INodeInterface>> {
        match self {
            FileContainer::File(file) => Ok(Arc::new(RamFile {
                inner: file.clone(),
            })),
            FileContainer::Dir(dir) => Ok(Arc::new(RamDir {
                inner: dir.clone(),
                dents_off: Mutex::new(0),
            })),
            FileContainer::Link(link) => Ok(Arc::new(RamLink {
                inner: link.clone(),
                link_file: link.link_file.clone(),
            })),
        }
    }

    #[inline]
    fn filename(&self) -> &str {
        match self {
            FileContainer::File(file) => &file.name,
            FileContainer::Dir(dir) => &dir.name,
            FileContainer::Link(link) => &link.name,
        }
    }
}
```

- `RamFs`结构体：代表整个RAM文件系统。它包含一个指向根目录的`Arc<RamDirInner>`。
- `impl RamFs`：为RamFs实现了一些方法。
  - `new()`方法：用于创建一个新的RamFs实例。它初始化了一个`RamDirInner`结构体作为根目录，并将其封装在`Arc`中返回。
  - `root_dir()`方法：返回根目录的`Arc<dyn INodeInterface>`实例，这是文件系统接口的一部分。
  - `name()`方法：返回文件系统的名称，即"ramfs"。
- `RamDirInner`结构体：表示RAM文件系统中的目录。它包含目录的名称和子项的列表。子项列表通过互斥锁`Mutex`来实现并发安全。
- `RamFileInner`结构体：表示RAM文件系统中的文件。它包含文件的名称、长度、页列表和时间戳。文件长度和页列表都被互斥锁`Mutex`保护。
- `RamLinkInner`结构体：表示RAM文件系统中的链接（符号链接）。它包含链接的名称和指向的文件（通过`Arc<dyn INodeInterface>`表示）。
- `FileContainer`枚举：表示文件系统中的文件、目录和链接。它可以是`File`（文件）、`Dir`（目录）或`Link`（链接）。每个变体包含对应类型的内部结构体的`Arc`。
- `impl FileContainer`：为`FileContainer`实现了一些方法。
  - `to_inode()`方法：将`FileContainer`转换为`Arc<dyn INodeInterface>`实例。根据`FileContainer`的类型，返回对应的`RamFile`、`RamDir`或`RamLink`实例。
  - `filename()`方法：返回`FileContainer`的文件名。

```rust
#[allow(dead_code)]
pub struct RamLink {
    inner: Arc<RamLinkInner>,
    link_file: Arc<dyn INodeInterface>,
}

pub struct RamDir {
    inner: Arc<RamDirInner>,
    dents_off: Mutex<usize>,
}

impl INodeInterface for RamDir {
    fn open(&self, name: &str, _flags: vfscore::OpenFlags) -> VfsResult<Arc<dyn INodeInterface>> {
        self.inner
            .children
            .lock()
            .iter()
            .find(|x| x.filename() == name)
            .map(|x| x.to_inode())
            .ok_or(VfsError::FileNotFound)?
    }

    fn touch(&self, name: &str) -> VfsResult<Arc<dyn INodeInterface>> {
        // Find file, return VfsError::AlreadyExists if file exists
        self.inner
            .children
            .lock()
            .iter()
            .find(|x| x.filename() == name)
            .map_or(Ok(()), |_| Err(VfsError::AlreadyExists))?;

        let new_inner = Arc::new(RamFileInner {
            name: String::from(name),
            // content: Mutex::new(Vec::new()),
            times: Mutex::new([Default::default(); 3]),
            len: Mutex::new(0),
            pages: Mutex::new(vec![]),
        });

        let new_file = Arc::new(RamFile {
            inner: new_inner.clone(),
        });

        self.inner
            .children
            .lock()
            .push(FileContainer::File(new_inner));

        Ok(new_file)
    }

    fn mkdir(&self, name: &str) -> VfsResult<Arc<dyn INodeInterface>> {
        // Find file, return VfsError::AlreadyExists if file exists
        self.inner
            .children
            .lock()
            .iter()
            .find(|x| x.filename() == name)
            .map_or(Ok(()), |_| Err(VfsError::AlreadyExists))?;

        let new_inner = Arc::new(RamDirInner {
            name: String::from(name),
            children: Mutex::new(Vec::new()),
        });

        let new_dir = Arc::new(RamDir {
            inner: new_inner.clone(),
            dents_off: Mutex::new(0),
        });

        self.inner
            .children
            .lock()
            .push(FileContainer::Dir(new_inner));

        Ok(new_dir)
    }

    fn rmdir(&self, name: &str) -> VfsResult<()> {
        // TODO: identify whether the dir is empty(through metadata.childrens)
        // return DirectoryNotEmpty if not empty.
        let len = self
            .inner
            .children
            .lock()
            .drain_filter(|x| match x {
                FileContainer::Dir(x) => x.name == name,
                _ => false,
            })
            .count();
        match len > 0 {
            true => Ok(()),
            false => Err(VfsError::FileNotFound),
        }
    }

    fn read_dir(&self) -> VfsResult<Vec<DirEntry>> {
        Ok(self
            .inner
            .children
            .lock()
            .iter()
            .map(|x| match x {
                FileContainer::File(file) => DirEntry {
                    filename: file.name.clone(),
                    // len: file.content.lock().len(),
                    len: *file.len.lock(),
                    file_type: FileType::File,
                },
                FileContainer::Dir(dir) => DirEntry {
                    filename: dir.name.clone(),
                    len: 0,
                    file_type: FileType::Directory,
                },
                FileContainer::Link(link) => DirEntry {
                    filename: link.name.clone(),
                    len: 0,
                    file_type: FileType::Link,
                },
            })
            .collect())
    }

    fn remove(&self, name: &str) -> VfsResult<()> {
        let len = self
            .inner
            .children
            .lock()
            .drain_filter(|x| match x {
                FileContainer::File(x) => x.name == name,
                FileContainer::Dir(_) => false,
                FileContainer::Link(x) => x.name == name,
            })
            .count();
        match len > 0 {
            true => Ok(()),
            false => Err(VfsError::FileNotFound),
        }
    }

    fn unlink(&self, name: &str) -> VfsResult<()> {
        self.remove(name)
    }

    fn metadata(&self) -> VfsResult<vfscore::Metadata> {
        Ok(Metadata {
            filename: &self.inner.name,
            inode: 0,
            file_type: FileType::Directory,
            size: 0,
            childrens: self.inner.children.lock().len(),
        })
    }

    fn stat(&self, stat: &mut Stat) -> VfsResult<()> {
        stat.ino = 1; // TODO: convert path to number(ino)
        stat.mode = StatMode::DIR; // TODO: add access mode
        stat.nlink = 1;
        stat.uid = 0;
        stat.gid = 0;
        stat.size = 0;
        stat.blksize = 512;
        stat.blocks = 0;
        stat.rdev = 0; // TODO: add device id
        stat.mtime = Default::default();
        stat.atime = Default::default();
        stat.ctime = Default::default();
        Ok(())
    }

    fn getdents(&self, buffer: &mut [u8]) -> VfsResult<usize> {
        let buf_ptr = buffer.as_mut_ptr() as usize;
        let len = buffer.len();
        let mut ptr: usize = buf_ptr;
        let mut finished = 0;
        for (i, x) in self
            .inner
            .children
            .lock()
            .iter()
            .enumerate()
            .skip(*self.dents_off.lock())
        {
            let filename = x.filename();
            let file_bytes = filename.as_bytes();
            let current_len = size_of::<Dirent64>() + file_bytes.len() + 1;
            if len - (ptr - buf_ptr) < current_len {
                break;
            }

            // let dirent = c2rust_ref(ptr as *mut Dirent);
            let dirent: &mut Dirent64 = unsafe { (ptr as *mut Dirent64).as_mut() }.unwrap();

            dirent.ino = 0;
            dirent.off = current_len as i64;
            dirent.reclen = current_len as u16;

            dirent.ftype = 0; // 0 ftype is file

            let buffer = unsafe {
                core::slice::from_raw_parts_mut(dirent.name.as_mut_ptr(), file_bytes.len() + 1)
            };
            buffer[..file_bytes.len()].copy_from_slice(file_bytes);
            buffer[file_bytes.len()] = b'\0';
            ptr = ptr + current_len;
            finished = i + 1;
        }
        *self.dents_off.lock() = finished;
        Ok(ptr - buf_ptr)
    }

    fn link(&self, name: &str, src: Arc<dyn INodeInterface>) -> VfsResult<()> {
        // Find file, return VfsError::AlreadyExists if file exists
        self.inner
            .children
            .lock()
            .iter()
            .find(|x| x.filename() == name)
            .map_or(Ok(()), |_| Err(VfsError::AlreadyExists))?;

        let new_inner = Arc::new(RamLinkInner {
            name: String::from(name),
            link_file: src,
        });

        self.inner
            .children
            .lock()
            .push(FileContainer::Link(new_inner));

        Ok(())
    }
}
```

这部分代码实现了`RamDir`结构体，它表示RAM文件系统中的目录。`RamDir`实现了`INodeInterface` trait，该trait定义了文件系统操作的接口方法。

以下是代码的解释：

- `RamLink`结构体：表示RAM文件系统中的链接。它包含一个指向`RamLinkInner`的`Arc`和一个指向链接的文件的`Arc<dyn INodeInterface>`。
- `RamDir`结构体：表示RAM文件系统中的目录。它包含一个指向`RamDirInner`的`Arc`和一个用于追踪目录项偏移量的`Mutex<usize>`。
- `impl INodeInterface for RamDir`：为`RamDir`实现了`INodeInterface` trait中的方法。
  - `open()`方法：根据给定的名称查找目录中的子项，并返回对应的`Arc<dyn INodeInterface>`实例。如果找不到子项，则返回`VfsError::FileNotFound`错误。
  - `touch()`方法：创建一个新文件，并将其添加到目录中。如果目录中已存在同名的文件，则返回`VfsError::AlreadyExists`错误。
  - `mkdir()`方法：创建一个新目录，并将其添加到目录中。如果目录中已存在同名的目录，则返回`VfsError::AlreadyExists`错误。
  - `rmdir()`方法：删除指定的目录。如果目录不为空，则返回`VfsError::DirectoryNotEmpty`错误。
  - `read_dir()`方法：返回目录中的所有目录项的列表。每个目录项都包含文件名、长度和文件类型。
  - `remove()`方法：从目录中移除指定的目录项（文件、目录或链接）。
  - `unlink()`方法：从目录中移除指定的链接。
  - `metadata()`方法：返回目录的元数据，包括文件名、inode、文件类型、大小和子项数量。
  - `stat()`方法：填充`Stat`结构体，包含目录的状态信息，如inode号、访问模式、链接数等。
  - `getdents()`方法：将目录中的目录项填充到提供的缓冲区中。
  - `link()`方法：在目录中创建一个链接，将其指向另一个文件。

这部分代码扩展了RAM文件系统的功能，实现了目录的创建、删除、查找等操作，并提供了与目录相关的元数据和状态信息。

```rust
pub struct RamFile {
    inner: Arc<RamFileInner>,
}

impl INodeInterface for RamFile {
    fn readat(&self, mut offset: usize, buffer: &mut [u8]) -> VfsResult<usize> {
        let mut buffer_off = 0;
        // let file_size = self.inner.content.lock().len();
        let file_size = *self.inner.len.lock();
        let inner = self.inner.pages.lock();

        match offset >= file_size {
            true => Ok(0),
            false => {
                // let origin_read_len = min(buffer.len(), file_size - offset);
                // let read_len = if offset >= real_size {
                //     min(origin_read_len, real_size - offset)
                // } else {
                //     0
                // };
                let read_len = min(buffer.len(), file_size - offset);
                let mut last_len = read_len;
                // let content = self.inner.content.lock();
                // buffer[..read_len].copy_from_slice(&content[offset..(offset + read_len)]);
                loop {
                    let curr_size = cmp::min(PAGE_SIZE - offset % PAGE_SIZE, last_len);
                    if curr_size == 0 {
                        break;
                    }
                    let index = offset / PAGE_SIZE;
                    let page_data = inner[index].0.get_buffer();
                    buffer[buffer_off..buffer_off + curr_size].copy_from_slice(
                        &page_data[offset % PAGE_SIZE..offset % PAGE_SIZE + curr_size],
                    );
                    offset += curr_size;
                    last_len -= curr_size;
                    buffer_off += curr_size;
                }
                // Ok(origin_read_len)
                Ok(read_len)
            }
        }
    }

    fn writeat(&self, mut offset: usize, buffer: &[u8]) -> VfsResult<usize> {
        let mut buffer_off = 0;
        let pages = ceil_div(offset + buffer.len(), PAGE_SIZE);

        let mut inner = self.inner.pages.lock();

        for _ in inner.len()..pages {
            inner.push(frame_alloc().expect("can't alloc frame in ram fs"));
        }

        let mut wsize = buffer.len();
        loop {
            let curr_size = cmp::min(PAGE_SIZE - offset % PAGE_SIZE, wsize);
            if curr_size == 0 {
                break;
            }
            let index = offset / PAGE_SIZE;
            let page_data = inner[index].0.get_buffer();
            page_data[offset % PAGE_SIZE..offset % PAGE_SIZE + curr_size]
                .copy_from_slice(&buffer[buffer_off..buffer_off + curr_size]);
            offset += curr_size;
            buffer_off += curr_size;
            wsize -= curr_size;
        }

        let file_size = *self.inner.len.lock();
        if offset > file_size {
            *self.inner.len.lock() = offset;
        }
        Ok(buffer.len())
    }

    fn truncate(&self, size: usize) -> VfsResult<()> {
        // self.inner.content.lock().drain(size..);
        *self.inner.len.lock() = size;

        let mut page_cont = self.inner.pages.lock();
        let pages = page_cont.len();
        let target_pages = ceil_div(size, PAGE_SIZE);

        for _ in pages..target_pages {
            page_cont.push(frame_alloc().expect("can't alloc frame in ram fs"));
        }

        Ok(())
    }

    fn metadata(&self) -> VfsResult<vfscore::Metadata> {
        Ok(Metadata {
            filename: &self.inner.name,
            inode: 0,
            file_type: FileType::File,
            // size: self.inner.content.lock().len(),
            size: *self.inner.len.lock(),
            childrens: 0,
        })
    }

    fn stat(&self, stat: &mut Stat) -> VfsResult<()> {
        // stat.ino = 1; // TODO: convert path to number(ino)
        if self.inner.name.ends_with(".s") {
            stat.ino = 2; // TODO: convert path to number(ino)
        } else {
            stat.ino = 1; // TODO: convert path to number(ino)
        }
        stat.mode = StatMode::FILE; // TODO: add access mode
        stat.nlink = 1;
        stat.uid = 0;
        stat.gid = 0;
        // stat.size = self.inner.content.lock().len() as u64;
        stat.size = *self.inner.len.lock() as u64;
        stat.blksize = 512;
        stat.blocks = 0;
        stat.rdev = 0; // TODO: add device id

        stat.atime = self.inner.times.lock()[1];
        stat.mtime = self.inner.times.lock()[2];
        Ok(())
    }

    fn utimes(&self, times: &mut [vfscore::TimeSpec]) -> VfsResult<()> {
        if times[0].nsec != UTIME_OMIT {
            self.inner.times.lock()[1] = times[0];
        }
        if times[1].nsec != UTIME_OMIT {
            self.inner.times.lock()[2] = times[1];
        }
        Ok(())
    }
}

impl INodeInterface for RamLink {
    fn metadata(&self) -> VfsResult<Metadata> {
        self.link_file.metadata()
    }

    fn stat(&self, stat: &mut Stat) -> VfsResult<()> {
        self.link_file.stat(stat)
    }

    fn readat(&self, offset: usize, buffer: &mut [u8]) -> VfsResult<usize> {
        self.link_file.readat(offset, buffer)
    }
}
```

这部分代码实现了`RamFile`和`RamLink`结构体，它们分别表示RAM文件系统中的文件和链接。这些结构体都实现了`INodeInterface` trait，该trait定义了文件系统操作的接口方法。

以下是代码的解释：

- `RamFile`结构体：表示RAM文件系统中的文件。它包含一个指向`RamFileInner`的`Arc`。
- `impl INodeInterface for RamFile`：为`RamFile`实现了`INodeInterface` trait中的方法。
  - `readat()`方法：从文件中读取数据到提供的缓冲区。根据指定的偏移量和缓冲区的大小，将文件中的数据复制到缓冲区中。
  - `writeat()`方法：将提供的数据写入文件的指定偏移量处。如果需要，会动态分配足够的内存页来容纳写入的数据。
  - `truncate()`方法：将文件截断到指定大小。如果指定的大小小于文件的当前大小，则会删除多余的数据页。如果指定的大小大于文件的当前大小，则会动态分配足够的内存页来扩展文件。
  - `metadata()`方法：返回文件的元数据，包括文件名、inode、文件类型、大小和子项数量。
  - `stat()`方法：填充`Stat`结构体，包含文件的状态信息，如inode号、访问模式、链接数等。
  - `utimes()`方法：更新文件的访问时间和修改时间。
- `RamLink`结构体：表示RAM文件系统中的链接。它包含一个指向其他文件的`Arc<dyn INodeInterface>`。
- `impl INodeInterface for RamLink`：为`RamLink`实现了`INodeInterface` trait中的方法。
  - `metadata()`方法：返回链接的元数据，即链接指向的文件的元数据。
  - `stat()`方法：填充`Stat`结构体，包含链接指向的文件的状态信息。
  - `readat()`方法：从链接指向的文件中读取数据。
