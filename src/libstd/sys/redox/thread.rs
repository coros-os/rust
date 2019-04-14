use crate::boxed::FnBox;
use crate::cmp;
use crate::ffi::CStr;
use crate::io;
use crate::mem;
use crate::ptr;
use crate::sys::os;
use crate::sys_common::thread::start_thread;
use crate::time::Duration;

//TODO: Move these to libc crate
const EINTR: libc::c_int = 4;
const EINVAL: libc::c_int = 22;
pub type pthread_t = *mut libc::c_void;
type pthread_attr_t = *mut libc::c_void;
extern "C" {
    fn nanosleep(
        rqtp: *const libc::timespec,
        rmtp: *mut libc::timespec
    ) -> libc::c_int;

    fn pthread_attr_destroy(attr: *mut pthread_attr_t) -> libc::c_int;
    fn pthread_attr_init(attr: *mut pthread_attr_t) -> libc::c_int;
    fn pthread_attr_setstacksize(
        attr: *mut pthread_attr_t,
        stacksize: libc::size_t
    ) -> libc::c_int;

    fn pthread_create(
        native: *mut pthread_t,
        attr: *const pthread_attr_t,
        f: extern "C" fn(_: *mut libc::c_void) -> *mut libc::c_void,
        value: *mut libc::c_void
    ) -> libc::c_int;
    fn pthread_detach(thread: pthread_t) -> libc::c_int;
    fn pthread_join(
        native: pthread_t,
        value: *mut *mut libc::c_void
    ) -> libc::c_int;

    fn sched_yield() -> libc::c_int;
}

pub const DEFAULT_MIN_STACK_SIZE: usize = 1024 * 1024;

pub struct Thread {
    id: pthread_t,
}

// Some platforms may have pthread_t as a pointer in which case we still want
// a thread to be Send/Sync
unsafe impl Send for Thread {}
unsafe impl Sync for Thread {}

impl Thread {
    // unsafe: see thread::Builder::spawn_unchecked for safety requirements
    pub unsafe fn new(stack: usize, p: Box<dyn FnBox()>)
                          -> io::Result<Thread> {
        let p = box p;
        let mut native: pthread_t = mem::zeroed();
        let mut attr: pthread_attr_t = mem::zeroed();
        assert_eq!(pthread_attr_init(&mut attr), 0);

        let stack_size = cmp::max(stack, 4096);

        match pthread_attr_setstacksize(&mut attr,
                                        stack_size) {
            0 => {}
            n => {
                assert_eq!(n, EINVAL);
                // EINVAL means |stack_size| is either too small or not a
                // multiple of the system page size.  Because it's definitely
                // >= PTHREAD_STACK_MIN, it must be an alignment issue.
                // Round up to the nearest page and try again.
                let page_size = os::page_size();
                let stack_size = (stack_size + page_size - 1) &
                                 (-(page_size as isize - 1) as usize - 1);
                assert_eq!(pthread_attr_setstacksize(&mut attr,
                                                           stack_size), 0);
            }
        };

        let ret = pthread_create(&mut native, &attr, thread_start,
                                       &*p as *const _ as *mut _);
        assert_eq!(pthread_attr_destroy(&mut attr), 0);

        return if ret != 0 {
            Err(io::Error::from_raw_os_error(ret))
        } else {
            mem::forget(p); // ownership passed to pthread_create
            Ok(Thread { id: native })
        };

        extern fn thread_start(main: *mut libc::c_void) -> *mut libc::c_void {
            unsafe { start_thread(main as *mut u8); }
            ptr::null_mut()
        }
    }

    pub fn yield_now() {
        let ret = unsafe { sched_yield() };
        debug_assert_eq!(ret, 0);
    }

    pub fn set_name(_name: &CStr) {
        // Redox cannot set thread name
    }

    pub fn sleep(dur: Duration) {
        let mut secs = dur.as_secs();
        let mut nsecs = dur.subsec_nanos() as _;

        // If we're awoken with a signal then the return value will be -1 and
        // nanosleep will fill in `ts` with the remaining time.
        unsafe {
            while secs > 0 || nsecs > 0 {
                let mut ts = libc::timespec {
                    tv_sec: cmp::min(libc::time_t::max_value() as u64, secs) as libc::time_t,
                    tv_nsec: nsecs,
                };
                secs -= ts.tv_sec as u64;
                if nanosleep(&ts, &mut ts) == -1 {
                    assert_eq!(os::errno(), EINTR);
                    secs += ts.tv_sec as u64;
                    nsecs = ts.tv_nsec;
                } else {
                    nsecs = 0;
                }
            }
        }
    }

    pub fn join(self) {
        unsafe {
            let ret = pthread_join(self.id, ptr::null_mut());
            mem::forget(self);
            assert!(ret == 0,
                    "failed to join thread: {}", io::Error::from_raw_os_error(ret));
        }
    }

    pub fn id(&self) -> pthread_t { self.id }

    pub fn into_id(self) -> pthread_t {
        let id = self.id;
        mem::forget(self);
        id
    }
}

impl Drop for Thread {
    fn drop(&mut self) {
        let ret = unsafe { pthread_detach(self.id) };
        debug_assert_eq!(ret, 0);
    }
}

pub mod guard {
    pub type Guard = !;
    pub unsafe fn current() -> Option<Guard> { None }
    pub unsafe fn init() -> Option<Guard> { None }
}
