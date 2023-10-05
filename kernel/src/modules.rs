#[allow(unused_imports)]
use kheader::macros::module_use;

module_use!(kvirtio);
#[cfg(feature = "nvme")]
module_use!(knvme);

module_use!(kgoldfish_rtc);

module_use!(general_plic);

#[cfg(feature = "board-k210")]
module_use!(k210_sdcard);
#[cfg(feature = "board-cv1811h")]
module_use!(kcvitek_sd);
