# KCov 实现

> TIPS: 具体请参考 syzkaller 分支.

Kcov 是一个代码覆盖率工具，用于评估测试套件对代码的覆盖程度。它可以帮助开发人员确定他们的测试用例是否足够全面，是否能够覆盖代码中的所有分支和路径。

Kcov 可以跟踪程序执行过程中哪些代码行被执行了，以及哪些代码行没有被执行到。它通过插桩（instrumentation）的方式，将额外的代码注入到被测试的程序中，以便收集覆盖率信息。

使用 Kcov，您可以获得以下信息：

代码行覆盖率：Kcov 可以告诉您在测试过程中有多少代码行被执行了，以及这些执行的代码行所占总代码行数的百分比。

分支覆盖率：Kcov 还可以提供关于代码中条件语句的分支覆盖信息。它可以告诉您在测试过程中有多少条件分支被覆盖了，以及这些覆盖的分支所占总分支数的百分比。

这些覆盖率信息可以帮助开发人员评估测试用例的质量，并确定是否需要编写更多的测试用例来增加代码覆盖率。

因此我们使用 kcov 的使用例程来作为测试用例，检测我们的 kcov 实现。

[https://www.kernel.org/doc/html/latest/dev-tools/kcov.html](https://www.kernel.org/doc/html/latest/dev-tools/kcov.html)

```c
#include <stdio.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <sys/types.h>
#include <sys/stat.h>
#include <sys/ioctl.h>
#include <sys/mman.h>
#include <unistd.h>
#include <fcntl.h>
#include <linux/types.h>

#define KCOV_INIT_TRACE                     _IOR('c', 1, unsigned long)
#define KCOV_ENABLE                 _IO('c', 100)
#define KCOV_DISABLE                        _IO('c', 101)
#define COVER_SIZE                  (64<<10)

#define KCOV_TRACE_PC  0
#define KCOV_TRACE_CMP 1

int main(int argc, char **argv)
{
    int fd;
    unsigned long *cover, n, i;

    /* A single fd descriptor allows coverage collection on a single
     * thread.
     */
    fd = open("/sys/kernel/debug/kcov", O_RDWR);
    if (fd == -1)
            perror("open"), exit(1);
    /* Setup trace mode and trace size. */
    if (ioctl(fd, KCOV_INIT_TRACE, COVER_SIZE))
            perror("ioctl"), exit(1);
    /* Mmap buffer shared between kernel- and user-space. */
    cover = (unsigned long*)mmap(NULL, COVER_SIZE * sizeof(unsigned long),
                                 PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0);
    if ((void*)cover == MAP_FAILED)
            perror("mmap"), exit(1);
    /* Enable coverage collection on the current thread. */
    if (ioctl(fd, KCOV_ENABLE, KCOV_TRACE_PC))
            perror("ioctl"), exit(1);
    /* Reset coverage from the tail of the ioctl() call. */
    __atomic_store_n(&cover[0], 0, __ATOMIC_RELAXED);
    /* Call the target syscall call. */
    read(-1, NULL, 0);
    /* Read number of PCs collected. */
    n = __atomic_load_n(&cover[0], __ATOMIC_RELAXED);
    for (i = 0; i < n; i++)
            printf("0x%lx\n", cover[i + 1]);
    /* Disable coverage collection for the current thread. After this call
     * coverage can be enabled for a different thread.
     */
    if (ioctl(fd, KCOV_DISABLE, 0))
            perror("ioctl"), exit(1);
    /* Free resources. */
    if (munmap(cover, COVER_SIZE * sizeof(unsigned long)))
            perror("munmap"), exit(1);
    if (close(fd))
            perror("close"), exit(1);
    return 0;
}
```

我们构造的 kcov 文件如下

```rust

pub struct Fuz{
    inner: Mutex<(usize, usize)>
}

impl Fuz {
    pub fn new() -> Arc<Fuz> {
        Arc::new(Self {
            inner: Mutex::new((0, 0))
        })
    }
}

impl INodeInterface for Fuz {
    fn readat(&self, _offset: usize, _buffer: &mut [u8]) -> vfscore::VfsResult<usize> {
        Ok(0)    
    }

    fn writeat(&self, _offset: usize, _buffer: &[u8]) -> vfscore::VfsResult<usize> {
        Ok(0)
    }

    fn ioctl(&self, command: usize, _arg: usize) -> vfscore::VfsResult<usize> {
        log::info!("ioctl: {} arg: {}", command, _arg);
        const ENABLE_FUZ: usize = 25444;
        const DISABLE_FUZ: usize = 25445;
        const INIT_FUZ_TRACE: usize = 2148033281;
        match command {
            ENABLE_FUZ => {
                let (addr, len) = *self.inner.lock();
                enable_fuz(addr, len);
                Ok(0)
            }
            DISABLE_FUZ => {
                disable_fuz();
                Ok(0)
            }
            INIT_FUZ_TRACE => {
                let (addr, len) = *self.inner.lock();
                init_fuz(addr, len);
                Ok(0)
            }
            _ => Err(vfscore::VfsError::NotSupported)
        }
    }

    fn after_mmap(&self, _addr: usize, _size: usize) -> vfscore::VfsResult<()> {
        log::info!("after_mmap: {}, size: {}", _addr, _size);
        *self.inner.lock() = (_addr, _size);
        Ok(())
    }
}
```

我们在内核的 init 线程中将 kcov 文件挂载在 ByteOS 的文件树上。然后利用 Fuz_records 进行 Fuz 数据的记录。

``` rust
pub static IS_FUZZING: AtomicBool = AtomicBool::new(false);

pub struct FuzzItem {
    pub ip: usize,
    pub tp: usize,
    pub arg1: usize,
    pub arg2: usize,
}

pub static FUZ_RECORDS: Mutex<(usize, usize)> = Mutex::new((0, 0));

pub fn enable_fuz(addr: usize, len: usize) {
    IS_FUZZING.store(true, core::sync::atomic::Ordering::Relaxed);
    log::info!("enable_fuzing: {:#x}, {:#x}", addr, len);
    *FUZ_RECORDS.lock() = (addr, len);
}

pub fn disable_fuz() {
    IS_FUZZING.store(false, core::sync::atomic::Ordering::Relaxed);
    *FUZ_RECORDS.lock() = (0, 0);
}

pub fn init_fuz(addr: usize, len: usize) {
    *FUZ_RECORDS.lock() = (addr, len);
}

pub fn write_fuz(data: &str) {
    if !IS_FUZZING.fetch_or(false, core::sync::atomic::Ordering::Relaxed) {
        return;
    }
    let ra: usize;
    unsafe {
        asm!("", out("ra") ra)
    }
    // log::error!("ra: {:#x}", ra);
    let (addr, len) = *FUZ_RECORDS.lock();
    if addr == 0 || len == 0 {
        return;
    }
    unsafe {
        let n = addr as *mut usize;
        let wptr = (addr as *mut usize).add(*n + 1);
        if wptr as usize >= addr + len {
            return;
        }
        wptr.write_volatile(ra);
        *n +=1;
    }
    println!("{}", data);
}

pub macro fuz() {
    fn f() {}
    fn type_name_of<T>(_: T) -> &'static str {
        core::any::type_name::<T>()
    }
    let name = type_name_of(f);
    $crate::kcov::write_fuz(&format!("{}\n{}: {}", &name[..name.len() - 16], file!(), line!()))
}
```

在内核的系统调用和需要 fuz 的地方加上 `fuz` 宏，然后在开启 `kcov` 后就可以进行记录。实现简单的 `kcov`.

我们在我们的 Makefile 文件中添加了 make addrline 指令，可以将 `kcov` 记录的地址进行转换，实现类似下面的输出。

```
SyS_read
fs/read_write.c:562
__fdget_pos
fs/file.c:774
__fget_light
fs/file.c:746
__fget_light
fs/file.c:750
__fget_light
fs/file.c:760
__fdget_pos
fs/file.c:784
SyS_read
fs/read_write.c:562
```