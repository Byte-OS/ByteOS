# procfs

procfs（/proc文件系统）是一种特殊的虚拟文件系统，它提供了一种以文件和目录的形式访问内核信息的方式。它是在许多类Unix操作系统中实现的，包括Linux。

procfs不是一个真正的文件系统，它没有对应的磁盘上的存储空间。相反，它是通过在内核中创建一个虚拟文件系统接口，将内核数据和状态以文件的形式呈现给用户空间。这些文件和目录可以被用户和应用程序访问和读取，提供了一种交互式地查看和监控内核信息的方式。

在procfs中，每个进程都有一个对应的目录，目录的名称是进程的PID（进程标识符）。在每个进程目录下，可以找到与该进程相关的各种信息，如进程状态、命令行参数、内存映射、打开的文件等。此外，procfs还提供了一些特殊的文件和目录，用于访问系统级别的信息，如CPU信息、内存信息、系统状态等。

通过读取procfs中的文件，用户和应用程序可以获取有关系统和进程的详细信息，监测系统性能，诊断问题，并与内核交互。这种以文件和目录的形式公开内核信息的设计使得procfs非常灵活和易于使用。

## 主要设计

```rust
impl INodeInterface for DevDirContainer {
    fn open(&self, name: &str, _flags: vfscore::OpenFlags) -> VfsResult<Arc<dyn INodeInterface>> {
        self.inner
            .map
            .get(name)
            .map(|x| x.clone())
            .ok_or(VfsError::FileNotFound)
    }

    fn read_dir(&self) -> VfsResult<Vec<DirEntry>> {
        Ok(self
            .inner
            .map
            .iter()
            .map(|(name, _)| DirEntry {
                filename: name.to_string(),
                len: 0,
                file_type: FileType::Device,
            })
            .collect())
    }

    fn stat(&self, stat: &mut vfscore::Stat) -> VfsResult<()> {
        ...
        Ok(())
    }

    fn metadata(&self) -> VfsResult<vfscore::Metadata> {
        Ok(vfscore::Metadata {
            filename: "dev",
            inode: 0,
            file_type: FileType::Directory,
            size: 0,
            childrens: self.inner.map.len(),
        })
    }

    fn getdents(&self, buffer: &mut [u8]) -> VfsResult<usize> {
        let buf_ptr = buffer.as_mut_ptr() as usize;
        let len = buffer.len();
        let mut ptr: usize = buf_ptr;
        let mut finished = 0;
        for (i, x) in self
            .inner
            .map
            .iter()
            .enumerate()
            .skip(*self.dents_off.lock())
        {
            let filename = x.0;
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
}
```

在`INodeInterface`的实现中，`open`函数用于打开一个文件或子目录，根据给定的名称在`inner.map`中查找相应的目录项，并返回对应的`INodeInterface`实例。如果找不到对应的目录项，则返回`FileNotFound`错误。

`read_dir`函数用于读取目录的内容，返回一个包含目录项信息的`Vec<DirEntry>`。在这个例子中，通过遍历`inner.map`中的目录项，为每个目录项创建一个`DirEntry`，并将其加入到结果`Vec`中。这里的目录项类型被指定为`FileType::Device`，长度为0。

`stat`函数用于获取文件或目录的元数据（stat信息），接受一个`Stat`结构体的引用，并在该结构体中填充相应的字段。在这个例子中，为了表示这是一个目录，`mode`字段被设置为`StatMode::DIR`，其他字段被赋予了一些默认值，与之前的代码段中的`MemInfo`和`Mounts`结构体的`stat`函数实现相同。

`metadata`函数用于获取文件或目录的元数据，返回一个`vfscore::Metadata`结构体，其中包含文件名、节点号、文件类型、大小和子目录项数量等信息。在这个例子中，文件名为"dev"，节点号为0，文件类型为`FileType::Directory`，大小为0，子目录项的数量为`inner.map`的长度。

`getdents`函数用于读取目录的目录项（dentry）。它接受一个缓冲区`buffer`，并将目录项的信息填充到缓冲区中。函数内部使用了`Dirent64`结构体表示目录项的格式。通过遍历`inner.map`中的目录项，将每个目录项的信息填充到缓冲区中，并更新指针`ptr`和偏移`finished`。最后，将更新后的偏移存储在`self.dents_off`中，并返回填充到缓冲区中的字节数。
