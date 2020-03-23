use crate::{Events, SysError};
use libc::{close, epoll_create1, epoll_ctl, epoll_wait};
use std::collections::HashMap;

impl From<u32> for Events {
    fn from(val: u32) -> Self {
        let mut events = Events::new();
        if (val & libc::EPOLLIN as u32) == libc::EPOLLIN as u32 {
            events = events.with_read();
        }
        if (val & libc::EPOLLOUT as u32) == libc::EPOLLOUT as u32 {
            events = events.with_write();
        }
        if (val & libc::EPOLLERR as u32) == libc::EPOLLERR as u32 {
            events = events.with_error();
        }
        events
    }
}

impl Into<u32> for Events {
    fn into(self) -> u32 {
        let mut events = 0u32;
        if self.has_read() {
            events |= libc::EPOLLIN as u32;
        }
        if self.has_write() {
            events |= libc::EPOLLOUT as u32;
        }
        if self.has_error() {
            events |= libc::EPOLLERR as u32;
        }
        events
    }
}

/// 定义文件 I/O 事件通知器。
#[derive(Debug)]
pub struct Poller {
    epoll_fd: i32,
    watches: HashMap<i32, Events>,
}

impl Default for Poller {
    fn default() -> Self {
        Self {
            epoll_fd: -1,
            watches: HashMap::new(),
        }
    }
}

impl Drop for Poller {
    fn drop(&mut self) {
        if self.epoll_fd > 0 {
            unsafe {
                close(self.epoll_fd);
            };
            self.epoll_fd = -1;
        }
    }
}

impl Poller {
    /// 创建一个新的 I/O 事件通知器。
    pub fn new() -> Self {
        let epoll_fd = unsafe { epoll_create1(0) };
        assert!(epoll_fd > 0, "epoll_create()");
        Self {
            epoll_fd,
            watches: HashMap::new(),
        }
    }

    /// 添加一个文件描述符到监视列表中。
    pub fn add(&mut self, fd: i32, events: Events) -> Result<(), SysError> {
        unsafe {
            let mut ev = libc::epoll_event {
                events: events.into(),
                u64: fd as u64,
            };
            let err = epoll_ctl(self.epoll_fd, libc::EPOLL_CTL_ADD, fd, &mut ev);
            if err < 0 {
                return Err(SysError::last());
            }
            self.watches.insert(fd, events);
            Ok(())
        }
    }

    /// 将一个文件描述符从监视列表中移除。
    pub fn remove(&mut self, fd: i32) -> Result<(), SysError> {
        if !self.watches.contains_key(&fd) {
            return Err(SysError::from(libc::ENOENT));
        }
        let err =
            unsafe { epoll_ctl(self.epoll_fd, libc::EPOLL_CTL_DEL, fd, std::ptr::null_mut()) };
        if err < 0 {
            Err(SysError::last())
        } else {
            self.watches.remove(&fd).unwrap();
            Ok(())
        }
    }

    /// 拉取所有被监测到的 I/O 事件。
    ///
    /// # Examples
    ///
    /// ```
    /// let mut poller = Poller::new();
    /// poller.add(0, Events::new().with_read());
    /// for (fd, events) in poller.pull_events(1000).unwrap().iter() {
    ///     println!("Fd={}, Events={}", fd, events);
    /// }
    /// ```
    pub fn pull_events(&self, timeout_ms: i32) -> Result<Vec<(i32, Events)>, SysError> {
        unsafe {
            let mut ev: Vec<libc::epoll_event> = Vec::with_capacity(self.watches.len());
            let nfds = epoll_wait(
                self.epoll_fd,
                ev.as_mut_ptr(),
                self.watches.len() as i32,
                timeout_ms,
            );
            if nfds < 0 {
                return Err(SysError::last());
            }
            ev.set_len(nfds as usize);
            Ok(ev
                .into_iter()
                .map(|x| (x.u64 as i32, Events::from(x.events)))
                .collect())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_poller() {
        unsafe {
            let cstr = std::ffi::CString::new("/proc/uptime").unwrap();
            let fd = libc::open(cstr.as_ptr(), libc::O_RDONLY);
            let mut poller = Poller::new();
            assert_eq!(poller.add(fd, Events::new().with_read()).is_ok(), true);
            for _ in 0..1000 {
                assert_eq!(poller.pull_events(1000).unwrap().len(), 1);
            }
            assert_eq!(poller.remove(fd).is_ok(), true);
            for _ in 0..1000 {
                assert_eq!(poller.add(fd, Events::new().with_read()).is_ok(), true);
                assert_eq!(poller.remove(fd).is_ok(), true);
            }
            libc::close(fd);
        }
    }
}
