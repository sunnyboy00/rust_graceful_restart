# Rust实现进程平滑重启
## 为试验性功能，由Go的平滑重启思路进行实现


> 项目仅支持linux系统

> 使用 `kill -USR1 pid` 来触发重启

```rust
n listen_signl(fd: RawFd) {
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
```