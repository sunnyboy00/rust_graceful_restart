use signal_hook::consts::*;
use std::{
    env::args,
    io::{Read, Write},
    net::{TcpListener, ToSocketAddrs},
    os::unix::{
        io::{AsRawFd, FromRawFd, RawFd},
        process::CommandExt,
    },
    thread,
};

static mut LISTENER: Option<TcpListener> = None;
fn main() {
    let pool = futures::executor::ThreadPool::new().unwrap();
    let mut is_init = true;
    let mut fd: Option<RawFd> = None;
    println!("{:#?}", args().collect::<Vec<String>>());
    let args = args().collect::<Vec<String>>();
    if args.len() > 1 {
        is_init = false;
        fd = match args[args.len() - 1].parse::<i32>() {
            Ok(i) => Some(i),
            Err(_) => None,
        };
    }
    let (_addr, listener) = start_reload("127.0.0.1:8080", is_init, fd).unwrap();
    let fork_fd = listener.as_raw_fd();
    //let mut f = unsafe { File::from_raw_fd(fd) };
    thread::spawn(move || {
        listen_signl(fork_fd);
    });

    loop {
        let (mut stream, _addr) = listener.accept().unwrap();
        pool.spawn_ok(async move {
            let mut buf = [0; 512];
            loop {
                println!("{}", is_init.clone());
                let buf_size = stream.read(&mut buf).unwrap_or(0);
                if buf_size == 0 {
                    return;
                }
                if stream.write(&buf[..buf_size]).is_err() {
                    return;
                };
                if stream.flush().is_err() {
                    return;
                };
            }
        });
    }
}
// 使用kill -USR1 pid重启后，子进程会继承父进程的socket文件描述符
fn start_reload<A>(
    addr: A,
    is_init: bool,
    fd: Option<RawFd>,
) -> Result<(String, TcpListener), &'static str>
where
    A: ToSocketAddrs,
{
    unsafe {
        LISTENER = if is_init {
            Some(std::net::TcpListener::bind(addr).unwrap())
        } else {
            Some(std::net::TcpListener::from_raw_fd(fd.unwrap()))
        };
        match LISTENER.take() {
            Some(listener) => Ok((listener.local_addr().unwrap().to_string(), listener)),
            None => Err("listener err"),
        }
    }
}

fn listen_signl(fd: RawFd) {
    let sig_s = vec![SIGUSR1];
    let mut signals = signal_hook::iterator::Signals::new(&sig_s).expect("信号监听失败");
    for signal in signals.forever() {
        if signal == SIGUSR1 {
            unsafe {
                let flags = libc::fcntl(fd, libc::F_GETFD);
                // 因为rust中创建的文件描述符会自动close，此处要进行调整，不然会报错 无效的描述符
                libc::fcntl(fd, libc::F_SETFD, flags & !libc::FD_CLOEXEC);
            }
            let _ = std::process::Command::new(args().collect::<Vec<String>>()[0].to_string())
                .args(std::env::args().skip(1))
                .arg(&fd.to_string())
                .envs(std::env::vars())
                .stdin(std::process::Stdio::inherit())
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .exec();
        }
    }
}
