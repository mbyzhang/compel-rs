use std::{
    ffi::{c_char, c_int, c_long},
    marker::PhantomData,
    os::fd::AsRawFd,
};

pub use compel_sys;
pub use syscalls;

use log::Level;
use syscalls::{SyscallArgs, Sysno};

#[derive(Debug, Clone, PartialEq, Copy)]
pub struct Error(&'static str, c_int);

impl Error {
    pub fn new(func: &'static str, res: c_int) -> Error {
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

pub struct ParasiteCtl<T, R>
where
    T: Send + Copy,
    R: Send + Copy,
{
    inner: *mut compel_sys::parasite_ctl,
    _marker_t: PhantomData<T>,
    _marker_r: PhantomData<R>,
}

impl<T, R> ParasiteCtl<T, R>
where
    T: Send + Copy,
    R: Send + Copy,
{
    pub fn prepare(pid: compel_sys::pid_t) -> Result<Self> {
        let ctl = ccheck_ptr!(compel_prepare(pid))?;

        Ok(ParasiteCtl {
            inner: ctl,
            _marker_t: PhantomData,
            _marker_r: PhantomData,
        })
    }

    pub fn infect(&mut self, nr_threads: usize) -> Result<()> {
        let args_size = std::mem::size_of::<T>();
        ccheck!(compel_infect(
            self.inner,
            nr_threads as _,
            args_size as _
        ))
        .map(|_| ())
    }

    pub fn as_mut_ptr(&mut self) -> *mut compel_sys::parasite_ctl {
        self.inner
    }

    pub fn infect_ctx_mut(&mut self) -> *mut compel_sys::infect_ctx {
        unsafe { compel_sys::compel_infect_ctx(self.inner) }
    }

    pub fn set_log_fd(&mut self, fd: impl AsRawFd) {
        unsafe {
            (*self.infect_ctx_mut()).log_fd = fd.as_raw_fd() as _;
        }
    }

    pub fn rpc_call_sync(&mut self, cmd: u32, args: T) -> Result<()> {
        let cmd = cmd + compel_sys::PARASITE_USER_CMDS;
        let args_size = std::mem::size_of::<T>();
        let args_dest = unsafe { compel_sys::compel_parasite_args_s(self.inner, args_size as _) };
        unsafe { *(args_dest as *mut T) = args }

        ccheck!(compel_rpc_call_sync(cmd, self.inner)).map(|_| ())
    }

    pub fn rpc_call_sync_ret(&mut self, cmd: u32, args: T) -> Result<R> {
        let cmd = cmd + compel_sys::PARASITE_USER_CMDS;
        let args_size = std::mem::size_of::<T>();
        let args_dest = unsafe { compel_sys::compel_parasite_args_s(self.inner, args_size as _) };
        unsafe { *(args_dest as *mut T) = args }

        ccheck!(compel_rpc_call_sync(cmd, self.inner)).map(|_| ())?;

        Ok(unsafe { *(args_dest as *const R) })
    }

    pub fn cure(self) -> Result<()> {
        ccheck!(compel_cure(self.inner)).map(|_| ())
    }

    pub fn syscall(&mut self, nr: Sysno, args: SyscallArgs) -> Result<i64> {
        let mut ret: c_long = 0;

        ccheck!(compel_syscall(
            self.inner,
            nr.id(),
            &mut ret as *mut _,
            args.arg0 as _,
            args.arg1 as _,
            args.arg2 as _,
            args.arg3 as _,
            args.arg4 as _,
            args.arg5 as _
        ))?;

        Ok(ret as _)
    }
}

impl<T: Send + Copy, R: Send + Copy> Drop for ParasiteCtl<T, R> {
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
