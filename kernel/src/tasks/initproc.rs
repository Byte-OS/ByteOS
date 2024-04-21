#![allow(dead_code)]
#![allow(unused_imports)]
use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use arch::debug::DebugConsole;
use executor::{current_task, yield_now, FUTURE_LIST, TASK_QUEUE};
use frame_allocator::get_free_pages;
use fs::{
    dentry::{dentry_open, dentry_root, DentryNode},
    get_filesystem, File, FileType, OpenFlags,
};
use log::debug;
use logging::get_char;
use vfscore::INodeInterface;

use crate::tasks::add_user_task;

use super::UserTask;

const LF: u8 = b'\n';
const CR: u8 = b'\r';
const DL: u8 = b'\x7f';
const BS: u8 = b'\x08';
const SPACE: u8 = b' ';

fn help() {
    println!("help");
    println!("ls");
    println!("clear");
    println!("exit");
}

fn list_files(file: File, space: usize) {
    for i in file.read_dir().expect("can't read dir") {
        println!("{:<3$}{} {}", "", i.filename, i.len, space);
        if i.file_type == FileType::Directory {
            list_files(
                file.open(&i.filename, OpenFlags::O_RDWR)
                    .expect("can't read dir"),
                space + 4,
            );
        }
    }
}

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
    TASK_QUEUE
        .lock()
        .iter()
        .for_each(|x| match x.clone().as_any().downcast::<UserTask>() {
            Ok(user_task) => {
                user_task.exit(0);
                FUTURE_LIST.lock().remove(&user_task.task_id);
            }
            Err(_) => {}
        });
    TASK_QUEUE
        .lock()
        .retain(|x| x.clone().as_any().downcast::<UserTask>().is_err());
}

async fn run_libc_test() -> bool {
    let commands = ["./runtest.exe -w entry-static.exe socket"];

    for i in commands {
        file_command(i).await
    }

    false
}

async fn run_all() -> bool {
    let commands = [
        "brk",
        "chdir",
        "clone",
        "close",
        "dup",
        "dup2",
        "execve",
        "exit",
        "fork",
        "fstat",
        "getcwd",
        "getpid",
        "getppid",
        "gettimeofday",
        "mkdir_",
        "mmap",
        "mount /dev/sda ./mnt",
        "munmap",
        "open",
        "times",
        "openat",
        "pipe",
        "read",
        "sleep",
        "umount /dev/sda ./mnt",
        "uname",
        "unlink",
        "wait",
        "waitpid",
        "getdents",
        "write",
        "yield",
    ];

    for i in commands {
        file_command(i).await
    }

    return true;
}

async fn file_command(cmd: &str) {
    let mut args: Vec<&str> = cmd.split(" ").filter(|x| *x != "").collect();
    debug!("cmd: {}  args: {:?}", cmd, args);
    let filename = args.drain(..1).last().unwrap();
    let filename = match filename.starts_with("/") {
        true => String::from(filename),
        false => String::from("/") + filename,
    };
    match dentry_open(dentry_root(), &filename, OpenFlags::O_RDONLY) {
        Ok(_) => {
            info!("exec: {}", filename);
            let mut args_extend = vec![filename.as_str()];
            args_extend.extend(args.into_iter());
            // args.into_iter().for_each(|x| args_extend.push(x));
            let task_id = add_user_task(&filename, args_extend, Vec::new()).await;
            loop {
                if TASK_QUEUE
                    .lock()
                    .iter()
                    .find(|x| x.get_task_id() == task_id)
                    .is_none()
                {
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

pub async fn command(cmd: &str) -> bool {
    match cmd.trim() {
        "" => {}
        "help" => help(),
        "ls" => list_files(
            dentry_open(dentry_root(), "/", OpenFlags::O_DIRECTORY)
                .expect("can't find mount point at .")
                .node
                .clone(),
            0,
        ),
        "clear" => clear(),
        "exit" => return true,
        "run_all" => return run_all().await,
        _ => file_command(cmd).await,
    }

    false
}

pub async fn simple_shell() {
    // simple command shell.
    let mut buffer = Vec::new();
    let mut new_line = true;
    loop {
        if new_line {
            print!("> ");
            new_line = false;
        }
        if let Some(c) = get_char() {
            match c as u8 {
                CR | LF => {
                    print!("\n");
                    let sign = command(&String::from_utf8_lossy(&buffer).to_string()).await;
                    if sign {
                        break;
                    }
                    buffer.clear();
                    new_line = true;
                }
                BS | DL => {
                    if buffer.len() > 0 {
                        buffer.pop();
                        DebugConsole::putchar(BS);
                        DebugConsole::putchar(SPACE);
                        DebugConsole::putchar(BS);
                    }
                }
                0..30 => {}
                _ => {
                    buffer.push(c as u8);
                    DebugConsole::putchar(c as u8);
                }
            }
        }
        yield_now().await;
    }
}

pub const USER_WORK_DIR: &'static str = "/";

pub async fn initproc() {
    // link files.
    // let rootfs = get_filesystem(0).root_dir();
    // let tmpfs = FileItem::fs_open("/home", OpenFlags::O_DIRECTORY).expect("can't open /home");
    // for file in rootfs.read_dir().expect("can't read files") {
    //     tmpfs
    //         .link(
    //             &file.filename,
    //             rootfs.open(&file.filename, OpenFlags::NONE).unwrap(),
    //         )
    //         .expect("can't link file to tmpfs");
    // }

    println!("start kernel tasks");
    // command("ls").await;
    // command("entry-static.exe crypt").await;
    // command("./runtest.exe -w entry-dynamic.exe dlopen").await;

    // let names = include_str!("../../../tools/testcase-step2/run-static.sh");
    // for (i, x) in names
    //     .split('\n')
    //     .filter(|x| !x.contains("clocale_mbfuncs") && !x.contains("pthread"))
    //     .enumerate()
    // {
    //     info!("No.{} started!", i);
    //     command(x).await;
    //     info!("No.{} finished!", i);
    // }

    // let names = include_str!("../../../tools/testcase-step2/run-static.sh");
    // for (i, x) in names
    //     .split('\n')
    //     .filter(|x| x.contains("clocale_mbfuncs") || x.contains("pthread"))
    //     .enumerate()
    // {
    //     info!("No.{} started!", i);
    //     command(x).await;
    //     info!("No.{} finished!", i);
    // }

    // let names = include_str!("../../../tools/testcase-step2/run-dynamic.sh");
    // for (i, x) in names
    //     .split('\n')
    //     .filter(|x| !x.contains("socket"))
    //     .enumerate()
    // {
    //     command(x).await;
    //     info!("No.{} finished!", i);
    // }

    // command("./runtest.exe -w entry-static.exe pthread_cancel").await;
    // command("./entry-static.exe pthread_cond_smasher").await;
    // command("./runtest.exe -w entry-static.exe pthread_cond_smasher").await;

    // command("test-fscanf").await;
    // command("./runtest.exe -w entry-static.exe argv").await;
    // command("entry-static.exe fscanf").await;
    // command(" busybox sh").await;
    // command("./a.out").await;

    // command("busybox echo run time-test").await;
    // command("time-test").await;

    // command("busybox echo run netperf_testcode.sh").await;
    // command("busybox sh netperf_testcode.sh").await;

    // command("busybox echo run busybox_testcode.sh").await;
    // command("busybox sh busybox_testcode.sh").await;

    // command("busybox echo run libctest_testcode.sh").await;
    // command("busybox sh libctest_testcode.sh").await;

    // command("busybox echo 123").await;
    command("qjs.static test.js").await;
    // command("qjs.static").await;
    command("busybox sh").await;
    // command("busybox echo run lua_testcode.sh").await;
    // command("busybox sh lua_testcode.sh").await;

    // command("busybox echo run cyclic_testcode.sh").await;
    // command("busybox sh cyclictest_testcode.sh").await;
    // kill_all_tasks().await;

    // command("libc-bench").await;

    // command("busybox echo run iperf_testcode.sh").await;
    // command("busybox sh iperf_testcode.sh").await;
    // kill_all_tasks().await;

    // command("busybox echo run iozone_testcode.sh").await;
    // command("busybox sh iozone_testcode.sh").await;

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

    // command("cyclictest -a -i 1000 -t1 -n -p99 -D 1s -q").await;
    // command("busybox mkdir test_dir").await;
    // command("busybox mv test_dir test").await;
    // command("./runtest.exe -w entry-static.exe pthread_cancel_points").await;
    // command("./runtest.exe -w entry-static.exe pthread_cancel").await;
    // command("./runtest.exe -w entry-static.exe pthread_condattr_setclock").await;
    // command("./runtest.exe -w entry-static.exe pthread_cond_smasher").await;
    // command("./runtest.exe -w entry-dynamic.exe tls_init").await;
    // command("./runtest.exe -w entry-dynamic.exe pthread_cancel_points").await;
    // command("./runtest.exe -w entry-static.exe utime").await;
    // command("./runtest.exe -w entry-static.exe clocale_mbfuncs").await;
    // command("./looper 2 ./multi.sh 1").await;
    // command("busybox sh ./multi.sh 1").await;
    // command("busybox sh ./tst.sh ./sort.src").await;
    // command("entry-dynamic.exe pthread_cancel_points").await;
    // command("bin/sh").await;
    // command("busybox sh").await;
    // command("cloudreve").await;
    // command("miniftpd").await;
    // command("/server_ftp.out").await;
    // command("http_server").await;
    // command("ssh-timeouts").await;
    // command("sshd").await;
    // command("./redis-server /redis.conf --loglevel verbose").await;
    // command("redis-cli-static").await;
    // command("bin/sh").await;
    // command("sshd").await;
    // command("busybox sh").await;
    // command("/bin/riscv64-linux-musl-gcc main.c").await;
    // command("busybox cp /tmp_home/a.out /").await;
    // command("busybox sh -c ./a.out").await;
    // command("cloudreve").await;
    // command("ssh-simple").await;
    // command("usr/bin/tcc -run main.c").await;
    // command("/bin/bash").await;
    // command("bin/bash lmbench_testcode.sh").await;
    // command("bin/busybox sh").await;
    // command("sqlite_test").await;
    // command("lmbench_all lat_syscall -P 1 null").await;
    // command("lmbench_all lat_syscall -P 1 read").await;
    // command("lmbench_all lat_syscall -P 1 write").await;
    // command("./lmbench_all lat_pipe -P 1").await;
    // command("bin/bash busybox_testcode.sh").await;
    // command("busybox sh lua_testcode.sh").await;
    // command("busybox sh lmbench_testcode.sh").await;
    // command("bin/busybox sh file_speed.sh").await;
    // command("redis-server redis.conf").await;
    // command("redis-cli-static").await;
    // command("sqlite_test").await;
    // command("sqlite_shell").await;
    // run_libc_test().await;
    // run_all().await;

    // command("helloworld").await;
    // command("filelist").await;
    // #[cfg(feature = "k210")]
    // command("busybox sh").await;
    // #[cfg(not(feature = "k210"))]
    // command("bin/sh").await;
    // simple_shell().await;
    // command("busybox").await;

    // switch_to_kernel_page_table();
    println!("!TEST FINISH!");
}
