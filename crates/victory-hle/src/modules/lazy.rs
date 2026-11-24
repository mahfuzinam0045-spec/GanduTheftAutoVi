//! Physical pages mapped into the title's own reservations, committed on first use.
//!
//! Ported from XCL (`crates/xcl/src/hle/kernelx/lazy.rs`).
//! A title maps its physical pages wherever it reserved address space and places
//! resources there — render targets and depth buffers too, which on the PC live
//! in video memory (`victory_gpu::vram`); the CPU never touches those pages.
//! Mapping them as reserved address space and committing each 64 KiB page on first
//! CPU touch via a vectored exception handler avoids doubling system commit.
//! Kernel calls (`ReadFile`, DirectStorage) do not fault into the handler, so
//! [`ensure`] commits their destination ranges beforehand.

use std::collections::BTreeSet;
use std::ffi::c_void;
use std::sync::{Once, RwLock};

use windows_sys::Win32::Foundation::{EXCEPTION_ACCESS_VIOLATION, NTSTATUS};
use windows_sys::Win32::System::Diagnostics::Debug::{
    AddVectoredExceptionHandler, EXCEPTION_POINTERS,
};
use windows_sys::Win32::System::Memory::{
    MEMORY_BASIC_INFORMATION, MEM_COMMIT, PAGE_EXECUTE_READ, PAGE_EXECUTE_READWRITE,
    PAGE_EXECUTE_WRITECOPY, PAGE_GUARD, PAGE_READONLY, PAGE_READWRITE, PAGE_WRITECOPY,
    VirtualAlloc, VirtualQuery,
};

const PAGE: usize = 0x1_0000;
const EXCEPTION_CONTINUE_EXECUTION: i32 = -1;
const EXCEPTION_CONTINUE_SEARCH: i32 = 0;

/// 64 KiB pages mapped but not committed yet.
static LAZY: RwLock<BTreeSet<usize>> = RwLock::new(BTreeSet::new());

fn install() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        // SAFETY: `handler` has the required `PVECTORED_EXCEPTION_HANDLER` signature.
        unsafe {
            AddVectoredExceptionHandler(1, Some(handler));
        }
    });
}

/// Commits `page` if it is currently marked lazy; returns `true` if committed.
unsafe fn commit(page: usize) -> bool {
    let mut g = LAZY.write().unwrap_or_else(|e| e.into_inner());
    if !g.remove(&page) {
        return false;
    }
    // SAFETY: `page` is a valid 64 KiB-aligned reserved section view.
    !unsafe { VirtualAlloc(page as *const c_void, PAGE, MEM_COMMIT, PAGE_READWRITE) }.is_null()
}

unsafe extern "system" fn handler(info: *mut EXCEPTION_POINTERS) -> i32 {
    // SAFETY: Windows passes valid `EXCEPTION_POINTERS` to vectored handlers.
    let r = unsafe { &*(*info).ExceptionRecord };
    if r.ExceptionCode != EXCEPTION_ACCESS_VIOLATION as NTSTATUS || r.NumberParameters < 2 {
        return EXCEPTION_CONTINUE_SEARCH;
    }
    let page = r.ExceptionInformation[1] & !(PAGE - 1);
    let is_lazy = LAZY.read().unwrap_or_else(|e| e.into_inner()).contains(&page);
    // SAFETY: `commit` and `allows` only query/commit virtual memory.
    let handled = unsafe {
        (is_lazy && commit(page)) || allows(r.ExceptionInformation[0], r.ExceptionInformation[1])
    };
    if handled {
        EXCEPTION_CONTINUE_EXECUTION
    } else {
        EXCEPTION_CONTINUE_SEARCH
    }
}

unsafe fn allows(access: usize, at: usize) -> bool {
    // SAFETY: `MEMORY_BASIC_INFORMATION` is a plain C struct where zero is valid.
    let mut m: MEMORY_BASIC_INFORMATION = unsafe { std::mem::zeroed() };
    // SAFETY: `m` is valid for `MEMORY_BASIC_INFORMATION`.
    if unsafe { VirtualQuery(at as *const c_void, &mut m, size_of_val(&m)) } == 0
        || m.State != MEM_COMMIT
        || m.Protect & PAGE_GUARD != 0
    {
        return false;
    }
    let writes = PAGE_READWRITE | PAGE_WRITECOPY | PAGE_EXECUTE_READWRITE | PAGE_EXECUTE_WRITECOPY;
    let reads = writes | PAGE_READONLY | PAGE_EXECUTE_READ;
    match access {
        0 => m.Protect & reads != 0,
        1 => m.Protect & writes != 0,
        _ => false,
    }
}

/// Marks a 64 KiB-aligned reserved page to be committed on first CPU use.
pub(crate) fn mark(page: usize) {
    install();
    LAZY.write().unwrap_or_else(|e| e.into_inner()).insert(page);
}

/// Unmarks a 64 KiB-aligned page (returns `true` if it was never committed).
pub(crate) fn unmark(page: usize) -> bool {
    LAZY.write().unwrap_or_else(|e| e.into_inner()).remove(&page)
}

/// Ensures every lazy page intersecting `[p, p + len)` is committed before a
/// kernel call or DMA write touches it.
pub fn ensure(p: usize, len: usize) {
    if len == 0 {
        return;
    }
    let pages: Vec<usize> = {
        let g = LAZY.read().unwrap_or_else(|e| e.into_inner());
        if g.is_empty() {
            return;
        }
        g.range((p & !(PAGE - 1))..p.saturating_add(len))
            .copied()
            .collect()
    };
    for page in pages {
        // SAFETY: `page` came from `LAZY`.
        unsafe { commit(page) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows_sys::Win32::System::Memory::{MEM_RELEASE, MEM_RESERVE, VirtualFree};

    #[test]
    fn a_lazy_page_is_committed_on_first_use() {
        // SAFETY: testing reserved VirtualAlloc memory with lazy commit.
        unsafe {
            let r = VirtualAlloc(std::ptr::null(), 2 * PAGE, MEM_RESERVE, PAGE_READWRITE) as usize;
            mark(r);
            mark(r + PAGE);
            assert_eq!(std::ptr::read_volatile((r + 8) as *const u32), 0);
            std::ptr::write_volatile((r + 8) as *mut u32, 7);
            assert_eq!(std::ptr::read_volatile((r + 8) as *const u32), 7);
            assert!(!unmark(r), "used: no longer lazy");
            ensure(r + PAGE + 4, 4);
            assert!(!LAZY.read().unwrap().contains(&(r + PAGE)));
            VirtualFree(r as *mut c_void, 0, MEM_RELEASE);
        }
    }

    #[test]
    fn threads_first_touching_one_lazy_page_together_all_go_on() {
        const THREADS: usize = 8;
        const PAGES: usize = 64;
        // SAFETY: testing concurrent first-touch on lazy reserved pages.
        unsafe {
            let r =
                VirtualAlloc(std::ptr::null(), PAGES * PAGE, MEM_RESERVE, PAGE_READWRITE) as usize;
            for i in 0..PAGES {
                mark(r + i * PAGE);
            }
            let barrier = std::sync::Arc::new(std::sync::Barrier::new(THREADS));
            let handles: Vec<_> = (0..THREADS)
                .map(|t| {
                    let barrier = barrier.clone();
                    std::thread::spawn(move || {
                        for i in 0..PAGES {
                            barrier.wait();
                            std::ptr::write_volatile(
                                (r + i * PAGE + t * 64) as *mut u64,
                                (i * THREADS + t) as u64,
                            );
                        }
                    })
                })
                .collect();
            for h in handles {
                h.join().unwrap();
            }
            for i in 0..PAGES {
                for t in 0..THREADS {
                    assert_eq!(
                        std::ptr::read_volatile((r + i * PAGE + t * 64) as *const u64),
                        (i * THREADS + t) as u64
                    );
                }
            }
            VirtualFree(r as *mut c_void, 0, MEM_RELEASE);
        }
    }
}
