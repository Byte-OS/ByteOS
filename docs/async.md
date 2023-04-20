# Async 编程

由于使用了 `Rust` 的 `async` 机制，所以动态的 `kill task` 也成为了必要，因此在编程的时候尽量避免在 `await` 之前对 `task` 进行枷锁，防止出现死锁的情况。 