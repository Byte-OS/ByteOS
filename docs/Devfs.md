# Devfs

DevFS（Device Filesystem）是一个虚拟文件系统，用于在Linux内核中表示和访问设备。它提供了一种以文件的形式来访问和操作设备的机制，使得设备可以像文件一样被读取、写入和控制。

在DevFS中，每个设备都被表示为文件节点，位于特定的目录结构中。这些文件节点可以通过访问标准的文件I/O接口来进行读取和写入。同时，DevFS还提供了一组特殊的文件节点，用于设备的控制和配置。

以下是DevFS的一些重要特点：

1. 设备表示：DevFS以一种层次结构的方式表示设备，将其组织为目录和文件的形式。每个设备都被表示为一个文件节点，可以通过路径来访问。
2. 设备访问：通过标准的文件I/O接口（如`open`、`read`、`write`、`close`）来访问和操作设备文件节点。这使得设备可以像普通文件一样进行读写操作。
3. 设备控制：除了设备文件节点，DevFS还提供了一些特殊的文件节点，用于设备的控制和配置。例如，可以使用这些文件节点来修改设备的属性、发送控制命令或查询设备状态。
4. 动态更新：DevFS可以根据系统中当前连接的设备动态更新其目录结构。当设备被插入或移除时，DevFS会相应地添加或删除相应的设备文件节点。
5. 虚拟文件系统：DevFS作为一个虚拟文件系统存在，不直接映射到物理存储设备上。它仅在内核中存在，并提供了一个抽象层，将设备表示为文件。

DevFS为设备提供了一种统一的访问机制，将设备的读写和控制操作以文件的形式暴露给用户空间。这种设计使得设备的操作更加灵活和易用，并与现有的文件和文件系统相关工具无缝集成。

以下是**ByteOS**中Devfs的实现：

```
├── Cargo.toml
└── src
    ├── cpu_dma_latency.rs	//cpuDMA延迟配置节点
    ├── lib.rs	//主要实现
    ├── null.rs	//空设备节点
    ├── rtc.rs	//RTC设备节点
    ├── sdx.rs	//存储设备节点
    ├── shm.rs	//共享内存节点
    ├── tty.rs	//终端设备访问节点
    ├── urandom.rs	//随机数读取节点
    └── zero.rs	//全零数据节点
```

CPU DMA延迟节点

```rust
pub struct CpuDmaLatency;

impl INodeInterface for CpuDmaLatency {
    fn readat(&self, _offset: usize, buffer: &mut [u8]) -> VfsResult<usize> {
        buffer.fill(0);
        if buffer.len() > 1 {
            buffer[0] = 1;
        }
        Ok(buffer.len())
    }

    fn writeat(&self, _offset: usize, buffer: &[u8]) -> VfsResult<usize> {
        Ok(buffer.len())
    }

    fn stat(&self, stat: &mut Stat) -> VfsResult<()> {
        ...
        Ok(())
    }
}
```

在该实现中，我们定义了一个名为`CpuDmaLatency`的结构体，用于表示CPU DMA延迟节点。这个结构体实现了`INodeInterface`特性，表明它是一个文件节点接口的实现。

在结构体的实现中，有几个方法被重写了：

1. `readat`方法：该方法用于从节点中读取数据。在这个具体的实现中，它将缓冲区（`buffer`）填充为零，并将首字节设置为1（如果缓冲区长度大于1）。最后，它返回读取的字节数。
2. `writeat`方法：该方法用于向节点中写入数据。在这个具体的实现中，它简单地返回写入的字节数。
3. `stat`方法：该方法用于获取节点的统计信息。在这个具体的实现中，它设置了一些`Stat`结构体中的字段，例如设备号（`dev`）、节点号（`ino`）、访问模式（`mode`）、链接计数（`nlink`）、用户ID（`uid`）、组ID（`gid`）等。注意，其中一些字段被标记为`TODO`，表示需要根据实际情况进行填充。最后，它返回一个空的`VfsResult`表示成功。

这个实现中的方法提供了对CPU DMA延迟节点的常见操作，包括读取、写入和获取统计信息等。

其余节点的实现与此类似。

以下是Devfs的主要实现

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
        ...
        Ok(ptr - buf_ptr)
    }
}
```

上述代码实现了`DevDirContainer`结构体对`INodeInterface` trait的方法进行了实现：

- `open()`方法用于打开指定名称的设备节点。它会在`inner`的映射中查找对应名称的设备节点，并返回它的克隆。如果找不到对应名称的设备节点，则返回`FileNotFound`错误。

- `read_dir()`方法用于读取目录内容。它会遍历`inner`的映射中的每个设备节点，并将设备节点的名称、长度、文件类型等信息封装为`DirEntry`结构体，最终返回一个包含所有`DirEntry`的`Vec`。此处由于是虚拟设备文件系统，因此每个设备节点的长度都被设置为0，并且文件类型为`FileType::Device`。

- `stat()`方法用于获取设备节点的元数据信息。它会将各种元数据信息填充到提供的`stat`结构体中，包括设备号、索引节点号、访问模式、链接数、所有者ID、大小、块大小、块数等。此处只是进行了一些默认值的填充。

- `metadata()`方法用于获取设备节点的元数据。它会返回一个`Metadata`结构体，包含文件名、索引节点号、文件类型、大小和子节点数量等信息。在这里，文件名被设置为"dev"，索引节点号为0，文件类型为`FileType::Directory`，大小为0，子节点数量为`inner.map`的长度。

- `getdents()`方法用于获取目录的目录项。它会将目录中的每个设备节点的信息填充到提供的缓冲区中，以`Dirent64`的结构体形式表示。每个`Dirent64`包含了设备节点的相关信息，如索引节点号、偏移量、记录长度、文件类型和文件名等。此处使用了`unsafe`代码块来操作原始指针，将设备节点的名称拷贝到缓冲区中，并更新指针位置和计数器。最后返回已填充数据的字节数。

这些方法的实现使得`DevDirContainer`成为一个可操作的虚拟设备文件系统目录，可以进行打开设备节点、读取目录内容、获取元数据和目录项等操作。

DevFS文件系统的实现具有以下亮点：

1. 设备节点映射：`DevDir`使用`BTreeMap`来存储设备节点的名称和对应的`INodeInterface`实现的Arc指针。这种映射关系允许快速查找和访问设备节点。
2. 文件系统接口：`DevFS`实现了`FileSystem` trait，提供了统一的文件系统操作接口。通过实现`root_dir()`、`name()`等方法，使得DevFS可以与其他文件系统一起使用，并符合通用的文件系统操作规范。
3. 高度可定制化：`DevDir`的构造函数提供了一种灵活的方式来添加设备节点。通过调用`add()`方法，可以向DevDir中动态添加新的设备节点，使得文件系统的内容可以根据需要进行定制和扩展。
4. 安全性：使用`Arc`和`Mutex`来处理并发访问的问题。`DevFS`和`DevDirContainer`都使用了`Arc`来进行引用计数和共享所有权，以确保线程安全性。同时，`DevDirContainer`中的`dents_off`字段使用了`Mutex`来处理目录项偏移量的并发访问。
5. 虚拟设备文件系统：DevFS是一个虚拟的设备文件系统，它提供了一种抽象的方式来表示设备节点。通过将不同类型的设备节点添加到`DevDir`中，可以模拟各种设备的存在和访问。

这些特点使得该DevFS文件系统的实现具有灵活性、可扩展性和安全性。它提供了一种简单而强大的方式来管理和访问设备节点，并能够方便地与其他文件系统进行集成和使用。