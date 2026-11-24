//! Console unified memory (UMA) for `victory-gpu`.
//!
//! Ported from XCL (`crates/xcl/src/gpu/uma.rs`). On the console, CPU and GPU
//! share memory and shaders reach it through plain addresses. We allocate up to
//! 4 windows of committed `VirtualAlloc` memory at a 16 GiB-aligned base (each
//! window `< 4 GiB`), which D3D12 opens via `ID3D12Device3::OpenExistingHeapFromAddress`.
//! Any buffer or allocation inside a committed window is directly GPU-visible
//! with zero CPU<->GPU copies.

use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};

use windows_sys::Win32::System::Memory::{
    MEM_COMMIT, MEM_RELEASE, MEM_RESERVE, PAGE_READWRITE, VirtualAlloc, VirtualFree,
};

/// Allocation granularity (matches `VirtualAlloc` 64 KiB).
pub const GRAIN: usize = 0x1_0000;
/// Largest region D3D12 opens as a heap (`4095 MiB`, since `4 GiB` is rejected).
pub const MAX_SIZE: usize = (4096 << 20) - GRAIN;
/// Distance between windows (`4 GiB`).
pub const WINDOW: usize = 1 << 32;
/// Maximum number of 4 GiB windows.
pub const MAX_WINDOWS: usize = 4;

/// Default committed window size (2 GiB) when `VICTORY_UMA_MB` is unset.
const DEFAULT_UMA_MB: usize = 2048;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum WindowKind {
    None,
    Committed(usize),
}

pub struct Uma {
    base: usize,
    kinds: [WindowKind; MAX_WINDOWS],
    win_allocs: Vec<usize>,
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    free: BTreeMap<usize, usize>,
    used: BTreeMap<usize, usize>,
}

// SAFETY: `Uma` owns its `VirtualAlloc` regions and synchronizes all
// allocator state behind `Mutex<Inner>`.
unsafe impl Send for Uma {}
// SAFETY: as above.
unsafe impl Sync for Uma {}

impl Uma {
    /// Allocates `size` bytes of committed UMA memory split across up to
    /// `MAX_WINDOWS` windows at a 16 GiB-aligned base.
    pub fn new(size: usize) -> Option<Uma> {
        let size = ((size + GRAIN - 1) & !(GRAIN - 1)).max(GRAIN);
        let n = size.div_ceil(MAX_SIZE).clamp(1, MAX_WINDOWS);
        let per_win = ((size.div_ceil(n) + GRAIN - 1) & !(GRAIN - 1)).min(MAX_SIZE);
        Self::with_windows(n, per_win)
    }

    fn with_windows(windows: usize, size: usize) -> Option<Uma> {
        let windows = windows.clamp(1, MAX_WINDOWS);
        let size = ((size + GRAIN - 1) & !(GRAIN - 1)).clamp(GRAIN, MAX_SIZE);
        let stride = (MAX_WINDOWS as usize) * WINDOW;
        let mut base = 16usize << 30;
        let mut win_allocs = Vec::with_capacity(windows);

        'search: loop {
            win_allocs.clear();
            let mut ok = true;
            for k in 0..windows {
                let want = base + k * WINDOW;
                // SAFETY: probing/allocating a fresh `VirtualAlloc` region at `want`.
                let p = unsafe {
                    VirtualAlloc(
                        want as *const _,
                        size,
                        MEM_RESERVE | MEM_COMMIT,
                        PAGE_READWRITE,
                    )
                };
                if p.is_null() || p as usize != want {
                    if !p.is_null() {
                        // SAFETY: `p` was just returned by `VirtualAlloc`.
                        unsafe { VirtualFree(p, 0, MEM_RELEASE) };
                    }
                    ok = false;
                    break;
                }
                win_allocs.push(p as usize);
            }
            if ok {
                break 'search;
            }
            for &p in &win_allocs {
                // SAFETY: `p` was allocated above in this loop.
                unsafe { VirtualFree(p as *mut _, 0, MEM_RELEASE) };
            }
            base = base.checked_add(stride)?;
            if base > (1usize << 45) {
                return None;
            }
        }

        let mut kinds = [WindowKind::None; MAX_WINDOWS];
        let mut free = BTreeMap::new();
        for (k, &addr) in win_allocs.iter().enumerate() {
            kinds[k] = WindowKind::Committed(size);
            free.insert(addr, size);
        }

        Some(Uma {
            base,
            kinds,
            win_allocs,
            inner: Mutex::new(Inner {
                free,
                used: BTreeMap::new(),
            }),
        })
    }

    /// Returns `(base, size)` for window `k` if committed, or `(0, 0)`.
    pub fn window(&self, k: usize) -> (usize, usize) {
        match self.kinds.get(k).copied().unwrap_or(WindowKind::None) {
            WindowKind::Committed(sz) => (self.base + k * WINDOW, sz),
            WindowKind::None => (0, 0),
        }
    }

    pub fn windows(&self) -> usize {
        self.kinds
            .iter()
            .filter(|k| matches!(k, WindowKind::Committed(_)))
            .count()
    }

    pub fn contains(&self, addr: usize, len: usize) -> bool {
        let Some(end) = addr.checked_add(len.max(1)) else {
            return false;
        };
        (0..MAX_WINDOWS).any(|k| {
            let (b, sz) = self.window(k);
            sz > 0 && addr >= b && end <= b + sz
        })
    }

    /// Allocates an `align`-aligned block of `size` bytes from UMA.
    pub fn alloc(&self, size: usize, align: usize) -> Option<*mut u8> {
        let size = size.max(1);
        let align = align.max(16).next_power_of_two();
        let mut g = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let found = g.free.iter().find_map(|(&start, &len)| {
            let aligned = (start.checked_add(align - 1)?) & !(align - 1);
            if aligned.checked_add(size).is_some_and(|e| e <= start + len) {
                Some((start, len, aligned))
            } else {
                None
            }
        });
        let (start, len, at) = found?;
        g.free.remove(&start);
        if at > start {
            g.free.insert(start, at - start);
        }
        let end = at + size;
        if end < start + len {
            g.free.insert(end, start + len - end);
        }
        g.used.insert(at, size);
        Some(at as *mut u8)
    }

    /// Frees a block previously returned by `Uma::alloc`. Returns `true` if owned.
    pub fn free(&self, ptr: *mut u8) -> bool {
        let addr = ptr as usize;
        let mut g = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let Some(size) = g.used.remove(&addr) else {
            return false;
        };
        // Coalesce with adjacent free blocks in the same window.
        let win = (addr - self.base) / WINDOW;
        let (win_base, win_size) = self.window(win);
        let mut start = addr;
        let mut len = size;

        if let Some((&prev_start, &prev_len)) = g.free.range(..start).next_back()
            && prev_start >= win_base
            && prev_start + prev_len == start
        {
            g.free.remove(&prev_start);
            start = prev_start;
            len += prev_len;
        }
        if let Some(&next_len) = g.free.get(&(start + len))
            && start + len + next_len <= win_base + win_size
        {
            g.free.remove(&(start + len));
            len += next_len;
        }
        g.free.insert(start, len);
        true
    }
}

impl Drop for Uma {
    fn drop(&mut self) {
        for &p in &self.win_allocs {
            // SAFETY: each `p` was allocated with `VirtualAlloc(MEM_RESERVE | MEM_COMMIT)`.
            unsafe { VirtualFree(p as *mut _, 0, MEM_RELEASE) };
        }
    }
}

static GLOBAL_UMA: OnceLock<Option<Uma>> = OnceLock::new();

/// Initializes the global UMA arena once (respecting `VICTORY_UMA_MB`, or `0` to disable).
pub fn init_runtime() -> Option<&'static Uma> {
    GLOBAL_UMA
        .get_or_init(|| {
            let mb = std::env::var("VICTORY_UMA_MB")
                .ok()
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(DEFAULT_UMA_MB);
            if mb == 0 {
                tracing::info!("UMA disabled via VICTORY_UMA_MB=0");
                return None;
            }
            let uma = Uma::new(mb << 20);
            match &uma {
                Some(u) => tracing::info!(
                    mb,
                    windows = u.windows(),
                    base = format_args!("{:#x}", u.base),
                    "UMA unified memory initialized"
                ),
                None => tracing::warn!(mb, "UMA initialization failed; using shadow buffers"),
            }
            uma
        })
        .as_ref()
}

/// Returns the global UMA instance if initialized.
pub fn get() -> Option<&'static Uma> {
    GLOBAL_UMA.get().and_then(|o| o.as_ref())
}

/// Allocates `size` bytes aligned to `align` from global UMA.
pub fn alloc(size: usize, align: usize) -> Option<*mut u8> {
    get()?.alloc(size, align)
}

/// Frees a pointer if it belongs to global UMA.
pub fn free(ptr: *mut u8) -> bool {
    if ptr.is_null() {
        return false;
    }
    get().is_some_and(|u| u.free(ptr))
}

/// Returns `true` if `[addr, addr + len)` lies within a committed UMA window.
pub fn contains(addr: usize, len: usize) -> bool {
    get().is_some_and(|u| u.contains(addr, len))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alloc_free_coalesce() {
        let u = Uma::new(4 << 20).expect("4 MiB UMA");
        let a = u.alloc(0x1000, 0x1000).expect("alloc a");
        let b = u.alloc(0x2000, 0x1000).expect("alloc b");
        assert!(u.contains(a as usize, 0x1000));
        assert!(u.contains(b as usize, 0x2000));
        assert!(u.free(a));
        assert!(u.free(b));
        let c = u.alloc(0x3000, 0x1000).expect("coalesced alloc");
        assert_eq!(c, a);
        assert!(u.free(c));
    }
}
