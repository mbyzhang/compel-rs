use std::ffi::{c_char, c_int};

pub use compel_sys;
use log::Level;

#[derive(Debug, Clone, PartialEq, Copy)]
pub struct Error(&'static str, std::ffi::c_int);

impl Error {
    pub fn new(func: &'static str, res: std::ffi::c_int) -> Error {
        Error(func, res)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

macro_rules! ccheck {
    ($f: ident ( $($x: expr),* ) ) => {{
        let r = unsafe { compel_sys::$f( $($x),* ) };
        if r < 0 { Err(Error::new(stringify!($f), r)) }
        else { Ok(r) }
    }}
}

macro_rules! ccheck_ptr {
    ($f: ident ( $($x: expr),* ) ) => {{
        let r = unsafe { compel_sys::$f( $($x),* ) };
        if r == std::ptr::null_mut() { Err(Error::new(stringify!($f), 0)) }
        else { Ok(r) }
    }}
}

pub struct ParasiteCtl {
    inner: *mut compel_sys::parasite_ctl,
}

impl ParasiteCtl {
    pub fn prepare(pid: compel_sys::pid_t) -> Result<Self> {
        let ctl = ccheck_ptr!(compel_prepare(pid))?;

        Ok(ParasiteCtl { inner: ctl })
    }

    pub fn infect(&mut self, nr_threads: usize, args_size: usize) -> Result<()> {
        ccheck!(compel_infect(
            self.inner,
            nr_threads as u64,
            args_size as u64
        ))
        .map(|_| ())
    }

    pub fn as_mut_ptr(&mut self) -> *mut compel_sys::parasite_ctl {
        self.inner
    }

    pub fn infect_ctx_mut(&mut self) -> *mut compel_sys::infect_ctx {
        unsafe { compel_sys::compel_infect_ctx(self.inner) }
    }

    pub fn set_log_fd(&mut self, fd: c_int) {
        unsafe { compel_sys::compel_infect_ctx_set_log_fd(self.infect_ctx_mut(), fd) };
    }

    pub fn rpc_call_sync<T: Copy>(&mut self, cmd: impl Into<u32>, arg: &T) -> Result<()> {
        let cmd = cmd.into() + compel_sys::PARASITE_USER_CMDS;
        let arg_size = std::mem::size_of::<T>();
        let arg_dest = unsafe { compel_sys::compel_parasite_args_s(self.inner, arg_size as u64) };
        unsafe { std::ptr::copy_nonoverlapping(arg, arg_dest as *mut _, 1) };

        ccheck!(compel_rpc_call_sync(cmd, self.inner)).map(|_| ())
    }

    pub fn cure(self) -> Result<()> {
        ccheck!(compel_cure(self.inner)).map(|_| ())
    }
}

impl Drop for ParasiteCtl {
    fn drop(&mut self) {
        // todo!()
    }
}

extern "C" fn print_compel_log(
    level: u32,
    fmt: *const c_char,
    parms: *mut compel_sys::__va_list_tag,
) {
    let s = unsafe { vsprintf::vsprintf(fmt, parms) }.expect("failed to print compel log");
    let s = s.trim_end();
    let level = match level {
        compel_sys::__compel_log_levels_COMPEL_LOG_DEBUG => Level::Debug,
        compel_sys::__compel_log_levels_COMPEL_LOG_INFO => Level::Info,
        compel_sys::__compel_log_levels_COMPEL_LOG_WARN => Level::Warn,
        compel_sys::__compel_log_levels_COMPEL_LOG_ERROR => Level::Error,
        compel_sys::__compel_log_levels_COMPEL_LOG_MSG => Level::Error,
        _ => Level::Trace,
    };
    log::log!(level, "{}", s);
}

pub fn log_init() {
    unsafe {
        compel_sys::compel_log_init(
            Some(print_compel_log),
            compel_sys::__compel_log_levels_COMPEL_LOG_DEBUG,
        );
    }
}
