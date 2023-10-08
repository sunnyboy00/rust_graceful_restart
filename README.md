# Rust实现进程平滑重启
## 
模拟 axum服务器的升级

如本程序名字为 rust_graceful_restart  ，将待升级的程序名字设定为 rust_graceful_restart1
执行
kill -USR1 pid

会从rust_graceful_restart 优雅退出，
重启 rust_graceful_restart1  实现优雅升级


> 项目仅支持linux系统

> 使用 `kill -USR1 pid` 来触发重启

```rust


// use std::time::Duration;

use std::{net::SocketAddr, str::FromStr};

//

use axum::{http::StatusCode, routing::get, Router};

use std::{
    env::args,
    net::TcpListener,
    os::unix::{
        io::{AsRawFd, FromRawFd, RawFd},
        process::CommandExt,
    },
    process,
};
use tokio::signal;



#[tokio::main]
async fn main() {
  
        let addr = SocketAddr::from_str("127.0.0.1:8080").unwrap();

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

   

    let listener = if is_init {
        std::net::TcpListener::bind(addr).unwrap()
    } else {
        println!("TcpListener::from_raw_fd");
        unsafe { std::net::TcpListener::from_raw_fd(fd.unwrap()) }
    };

    let fork_fd = listener.as_raw_fd();


        let app = Router::new().route("/", get(|| async { "Hello, World!" }));

        axum::Server::from_tcp(listener)
            .expect("listen from_tcp error")
            .serve(app.into_make_service())
            .with_graceful_shutdown(shutdown_signal( fork_fd))
            .await
            .unwrap();
        
   // });

    println!("main quit !!!")
}

async fn shutdown_signal( fd: RawFd) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

   
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    let sigusr1 = async {
        signal::unix::signal(signal::unix::SignalKind::user_defined1())
            .expect("failed to install signal handler user_defined1")
            .recv()
            .await;
    };
  

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
        _ = sigusr1 => {
            unsafe {
                let flags = libc::fcntl(fd, libc::F_GETFD);
                // 因为rust中创建的文件描述符会自动close，此处要进行调整，不然会报错 无效的描述符
                libc::fcntl(fd, libc::F_SETFD, flags & !libc::FD_CLOEXEC);
            }

            println!(" get signal usr1");
            let cmdname = if args().collect::<Vec<String>>()[0].contains("1") {
                args().collect::<Vec<String>>()[0].trim_end_matches("1").to_string()
            } else {
                args().collect::<Vec<String>>()[0].to_string() + "1"
            };

            let _ = std::process::Command::new(cmdname)
                .arg(&fd.to_string())

                .envs(std::env::vars())
                .stdin(std::process::Stdio::inherit())
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .spawn();

                println!(" signal exit");
        },

    }

    println!("signal received, starting graceful shutdown");
}

async fn handle_404() -> (StatusCode, &'static str) {
    (StatusCode::NOT_FOUND, "Not found")
}

