#![allow(dead_code)]
#![allow(unused_imports)]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use devices::utils::get_char;
use executor::{current_task, release_task, task::TaskType, tid2task, yield_now, TASK_MAP};
use fs::{file::File, FileType, OpenFlags};
use log::debug;
use polyhal::{debug_console::DebugConsole, instruction::shutdown};
use vfscore::INodeInterface;

use crate::tasks::add_user_task;

use super::UserTask;

fn clear() {
    DebugConsole::putchar(0x1b);
    DebugConsole::putchar(0x5b);
    DebugConsole::putchar(0x48);
    DebugConsole::putchar(0x1b);
    DebugConsole::putchar(0x5b);
    DebugConsole::putchar(0x32);
    DebugConsole::putchar(0x4a);
}

async fn kill_all_tasks() {
    TASK_MAP.lock().values().into_iter().for_each(|task| {
        task.upgrade().inspect(|x| {
            if x.get_task_type() == TaskType::MonolithicTask {
                x.exit(100)
            }
        });
    });
}

async fn command(cmd: &str) {
    let mut args: Vec<&str> = cmd.split(" ").filter(|x| *x != "").collect();
    debug!("cmd: {}  args: {:?}", cmd, args);
    let filename = args.drain(..1).last().unwrap();
    match File::open(filename.into(), OpenFlags::O_RDONLY) {
        Ok(_) => {
            info!("exec: {}", filename);
            let mut args_extend = vec![filename];
            args_extend.extend(args.into_iter());
            let task_id = add_user_task(&filename, args_extend, Vec::new()).await;
            let task = tid2task(task_id).unwrap();
            loop {
                if task.exit_code().is_some() {
                    release_task(task_id);
                    break;
                }
                yield_now().await;
            }
            // syscall(SYS_WAIT4, [0,0,0,0,0,0,0])
            //     .await
            //     .expect("can't wait a pid");
        }
        Err(_) => {
            println!("unknown command: {}", cmd);
        }
    }
}

pub async fn initproc() {
    println!("start kernel tasks");
    // command("./runtest.exe -w entry-dynamic.exe argv").await;
    // command("./entry-dynamic.exe argv").await;
    // command("busybox echo run time-test").await;
    // command("time-test").await;

    // command("busybox sh basic/run-all.sh").await;

    // command("busybox echo run netperf_testcode.sh").await;
    // command("busybox sh netperf_testcode.sh").await;

    // command("busybox echo run busybox_testcode.sh").await;
    // command("busybox sh busybox_testcode.sh").await;

    // command("busybox echo run libctest_testcode.sh").await;
    // command("busybox sh libctest_testcode.sh").await;
    // command("runtest.exe -w entry-static.exe utime").await;
    // command("busybox ln -s /busybox /bin/cat").await;
    // command("./bin/cat libctest_testcode.sh").await;
    // command("busybox ls -l /bin").await;
    // command("busybox ln -s /busybox /bin/ln").await;
    // command("busybox ln -s /busybox /bin/wget").await;
    // command("busybox ln -s /busybox /bin/xz").await;
    // command("busybox ls -l /bin").await;
    // command("busybox sh init.sh").await;
    // command("busybox ls -l /bin").await;

    command("busybox sh").await;
    // command("busybox echo run lua_testcode.sh").await;
    // command("busybox sh lua_testcode.sh").await;

    // command("busybox init").await;
    // command("busybox sh").await;
    // command("busybox sh init.sh").await;

    // command("busybox echo run cyclic_testcode.sh").await;
    // command("busybox sh cyclictest_testcode.sh").await;
    // kill_all_tasks().await;

    // command("libc-bench").await;

    // command("busybox echo run iperf_testcode.sh").await;
    // command("busybox sh iperf_testcode.sh").await;
    // kill_all_tasks().await;

    // command("busybox echo run iozone_testcode.sh").await;
    // command("busybox sh iozone_testcode.sh ").await;

    // command("busybox echo run lmbench_testcode.sh").await;
    // command("busybox sh lmbench_testcode.sh").await;

    // command("busybox echo run unixbench_testcode.sh").await;
    // command("busybox sh unixbench_testcode.sh").await;

    // command("copy-file-range-test-1").await;
    // command("copy-file-range-test-2").await;
    // command("copy-file-range-test-3").await;
    // command("copy-file-range-test-4").await;
    // command("interrupts-test-1").await;
    // command("interrupts-test-2").await;

    // switch_to_kernel_page_table();
    println!("!TEST FINISH!");

    // Shutdown if there just have blankkernel task.
    if TASK_MAP
        .lock()
        .values()
        .find(|x| {
            x.upgrade()
                .map(|x| x.get_task_type() != TaskType::BlankKernel)
                .unwrap_or(false)
        })
        .is_none()
    {
        shutdown();
    }
}
