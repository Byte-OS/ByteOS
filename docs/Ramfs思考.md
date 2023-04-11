# Ramfs 思考

## 思路是什么？

Ramfs 为例： 兼顾 `mount`, 无需设置 `mount point` 吗？
只需要在初始化的时候设置好相对于文件系统的 path 即可，

那如何判断 `mount` 的问题？
可以在 `fs` `item` 里指向文件系统 本身，然后做到这个？

觉得可以 所以 `vfs::FileSystem::root_dir`  `mount_point` 添加是不是有点多余
但是如果有多个挂载点的话怎么办
所以也许 `mount_point` 可以设置成一个 `Arc` 的信息，然后在每一个节点保存

如果没有 `mount_point` 怎么办？直接设置一个？

`Linux` 的 `/var/run` 机制是怎么做到的？到底是挂载了还是没挂载？

利用 `root_dir` 传递参数这种机制，可以保证在 `fatfs-shim` 里取消 `fatfs` `new` 时传递的文件系统相关参数
改为在 `root_dir` 里传递一个 `MountedInfo` 结构，里面包一层 `Arc` 
目前构想的结构
```rust
struct MountedInfo {
    path: Arc<String>,
    fs: Arc<dyn FileSystem>
}
```

利用这种机制就可以 在 `open` 文件的时候判断是否有文件夹 `mounted` 在这个文件夹下

但是 `offset` ? 这个是问题

所以还需要 `inner` 这个机制
文件需要 `inner` 和 `container`(或者就是文件名)
`inner` 保存在 `RamFsDir` 里，然后创建文件时直接创建一层壳，`inner` 直接 `clone`
`dir` 同理
但是出现一个问题，原来在 `dir` 的保存信息是 `Vec<Arc<dyn INodeInterface>>`,
更新结构过后需要设计一种新的结构 或者使用 `enum`
```rust
pub enum RamItem {
    File(RamFsFile),
    Dir(RamFsDir),
}
```
