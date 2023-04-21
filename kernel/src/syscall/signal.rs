use log::debug;

use super::consts::LinuxError;

/// TODO: finish sigtimedwait
pub async fn sys_sigtimedwait() -> Result<usize, LinuxError> {
    debug!("sys_sigtimedwait @ ");
    // Err(LinuxError::EAGAIN)
    Ok(0)
}
