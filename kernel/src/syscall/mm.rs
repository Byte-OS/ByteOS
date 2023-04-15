use executor::current_task;
use log::debug;

use super::consts::LinuxError;

pub async fn sys_brk(addr: isize) -> Result<usize, LinuxError> {
    debug!("sys_brk @ increment: {}", addr);
    let user_task = current_task().as_user_task().unwrap();
    if addr == 0 {
        Ok(user_task.heap())
    } else {
        debug!("alloc pos: {}", addr - user_task.heap() as isize);
        Ok(user_task.sbrk(addr - user_task.heap() as isize))
    }
}
