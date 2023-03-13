use std::{
    fs::File,
    io::{BufRead, BufReader},
    os::fd::{FromRawFd, RawFd},
    thread::{self, JoinHandle},
};

pub struct PieLogger {
    pub fd_write: RawFd,
    join_handle: Option<JoinHandle<()>>,
}

impl PieLogger {
    pub fn new() -> Self {
        let (fd_read, fd_write) = nix::unistd::pipe().expect("failed to pipe");

        let join_handle = thread::spawn(move || {
            let file = unsafe { File::from_raw_fd(fd_read) };
            let reader = BufReader::new(file);
            for line in reader.lines() {
                match line {
                    Ok(s) => {
                        log::log!(target: "pie", log::Level::Debug, "{}", s.trim_start_matches("pie: "))
                    }
                    Err(e) => {
                        log::error!("error reading stream: {:?}", e);
                        break;
                    }
                }
            }
        });
        Self {
            fd_write,
            join_handle: Some(join_handle),
        }
    }
}

impl Drop for PieLogger {
    fn drop(&mut self) {
        nix::unistd::close(self.fd_write).expect("failed to close write");
        self.join_handle.take().unwrap().join().unwrap()
    }
}
