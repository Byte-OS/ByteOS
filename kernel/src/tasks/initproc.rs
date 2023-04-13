use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use arch::{console_getchar, console_putchar};
use executor::yield_now;
use fs::{mount::open, File, FileType, OpenFlags};

use crate::syscall::{consts::SYS_EXECVE, syscall};

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
    console_putchar(0x1b);
    console_putchar(0x5b);
    console_putchar(0x48);
    console_putchar(0x1b);
    console_putchar(0x5b);
    console_putchar(0x32);
    console_putchar(0x4a);
}

async fn brk() {
    // pub async fn exec_with_process<'a>(task: Arc<dyn AsyncTask>, path: &'a str, args: Vec<&'a str>)
    // exec_with_process(current_task(), "/brk", Vec::new())
    //     .await
    //     .expect("can't exec file");
    let filename = "/brk\0";
    // syscall(SYS_EXECVE, filename.as_ptr() as _).await;
    let t = filename.as_ptr() as usize;
    syscall(SYS_EXECVE, [t, 0, 0, 0, 0, 0, 0])
        .await
        .expect("can't call execve");
}

pub async fn command(cmd: &str) -> bool {
    match cmd {
        "help" => help(),
        "ls" => list_files(open("/").expect("can't find mount point at ."), 0),
        "clear" => clear(),
        "exit" => return true,
        "brk" => brk().await,
        _ => println!("unknown command: {}", cmd),
    }

    false
}

pub async fn initproc() {
    let mut buffer = Vec::new();
    // let mut buffer = [0u8; 30];
    let mut new_line = true;
    loop {
        if new_line {
            print!("> ");
            new_line = false;
        }
        let c = console_getchar();
        if c as i8 != -1 {
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
                        console_putchar(BS);
                        console_putchar(SPACE);
                        console_putchar(BS);
                    }
                }
                0..30 => {}
                _ => {
                    buffer.push(c as u8);
                    console_putchar(c as u8);
                }
            }
        }
        yield_now().await;
    }
}
