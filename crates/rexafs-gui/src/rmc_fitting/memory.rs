//! Desktop cache policy introduced in 0.2.12. These are resource heuristics, not physics.
use serde::{Deserialize, Serialize};

pub const MIB: usize = 1024 * 1024;
const GIB: usize = 1024 * MIB;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemorySnapshot {
    /// Installed physical memory in bytes.
    pub total: usize,
    /// Available or reclaimable physical memory in bytes, as estimated by the OS.
    pub available: usize,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CacheBudget {
    /// Maximum retained scattering-snapshot payload, in bytes.
    pub bytes: usize,
    /// True for an automatically resolved limit; false for a manual override.
    pub automatic: bool,
    /// Latest native memory query, or None when the fallback is in use.
    pub memory: Option<MemorySnapshot>,
}

/// Automatic policy: reserve up to 1 GiB, then permit a quarter of the remaining
/// available-plus-owned cache memory, capped at a quarter of physical RAM and
/// 8 GiB. Adding retained cache bytes prevents the cache's own growth from being
/// mistaken for pressure from another application. This is an empirical rexafs
/// policy; the payload limit does not bound total process memory.
pub fn budget(
    manual_mib: Option<usize>,
    memory: Option<MemorySnapshot>,
    cached: usize,
) -> CacheBudget {
    let bytes = if let Some(mib) = manual_mib {
        mib.saturating_mul(MIB)
    } else if let Some(memory) = memory {
        let pool = memory.available.saturating_add(cached).min(memory.total);
        let reserve = (memory.total / 8).min(GIB);
        (pool.saturating_sub(reserve) / 4)
            .min(memory.total / 4)
            .min(8 * GIB)
            / MIB
            * MIB
    } else {
        256 * MIB
    };
    CacheBudget {
        bytes,
        automatic: manual_mib.is_none(),
        memory,
    }
}

/// Avoid reacting to small fluctuations; a change of at least 25% or 64 MiB
/// is applied. Zero always applies so an automatic cache can release its payload
/// under pressure. The calculator enforces the request at a batch boundary.
pub fn should_resize(current: usize, next: usize) -> bool {
    current != next
        && (next == 0
            || current == 0
            || current.abs_diff(next) >= (current / 4).min(64 * MIB).max(MIB))
}

pub fn validate_manual(mib: Option<usize>) -> Result<(), String> {
    if mib.is_some_and(|mib| !(1..=1_048_576).contains(&mib)) {
        return Err("Choose 1–1,048,576 MiB of cache memory, or Auto.".into());
    }
    Ok(())
}

/// Resolve the path-count guard once when submitting a new run. Auto reserves
/// the same quarter of available memory as the initial cache policy, separately
/// capped at 2 GiB, and allows 512 bytes per path. This empirical allowance covers
/// path identities and reverse atom indices, not electronic tables or total
/// process memory. It is a capacity heuristic, not a measured allocation bound.
/// Failed memory queries retain the historical one-million-path guard. The
/// resolved value is saved with the request so resume does not change identity.
pub fn catalogue_limit(manual: Option<usize>, memory: Option<MemorySnapshot>) -> usize {
    manual.unwrap_or_else(|| {
        if memory.is_none() {
            1_000_000
        } else {
            (budget(None, memory, 0).bytes.min(2 * GIB) / 512).max(1)
        }
    })
}

pub fn validate_catalogue_limit(limit: Option<usize>) -> Result<(), String> {
    if limit.is_some_and(|limit| !(1..=100_000_000).contains(&limit)) {
        return Err(
            "Choose 1–100,000,000 catalogue paths, or Auto. This limit applies to new runs.".into(),
        );
    }
    Ok(())
}

fn checked(total: u64, available: u64) -> Option<MemorySnapshot> {
    let total = usize::try_from(total).ok()?;
    let available = usize::try_from(available).ok()?;
    (total > 0).then_some(MemorySnapshot {
        total,
        available: available.min(total),
    })
}

#[cfg(any(target_os = "linux", test))]
fn linux_memory(text: &str) -> Option<MemorySnapshot> {
    let value = |name| {
        let line = text.lines().find(|line| line.starts_with(name))?;
        let mut fields = line.split_whitespace();
        fields.next()?;
        let kib = fields.next()?.parse::<u64>().ok()?;
        (fields.next()? == "kB").then_some(())?;
        kib.checked_mul(1024)
    };
    checked(value("MemTotal:")?, value("MemAvailable:")?)
}

/// Read physical-memory counters. A failed or unsupported query is explicit None;
/// callers retain the documented 256 MiB fallback and can show that limitation.
#[cfg(target_os = "linux")]
pub fn available_memory() -> Option<MemorySnapshot> {
    linux_memory(&std::fs::read_to_string("/proc/meminfo").ok()?)
}

#[cfg(target_os = "windows")]
pub fn available_memory() -> Option<MemorySnapshot> {
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    let mut status = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    // The correctly sized structure is writable for the duration of the call.
    unsafe {
        GlobalMemoryStatusEx(&mut status).ok()?;
    }
    checked(status.ullTotalPhys, status.ullAvailPhys)
}

#[cfg(target_os = "macos")]
#[allow(deprecated)] // libc retains the native Mach interfaces used by this query.
pub fn available_memory() -> Option<MemorySnapshot> {
    unsafe extern "C" {
        fn mach_port_deallocate(
            task: libc::mach_port_t,
            name: libc::mach_port_t,
        ) -> libc::kern_return_t;
    }
    let mut total = 0u64;
    let mut len = std::mem::size_of::<u64>();
    // Native calls receive initialized, correctly sized buffers. The host send
    // right is released after every query, including a failed statistics call.
    unsafe {
        if libc::sysctlbyname(
            c"hw.memsize".as_ptr(),
            (&mut total as *mut u64).cast(),
            &mut len,
            std::ptr::null_mut(),
            0,
        ) != 0
        {
            return None;
        }
        let host = libc::mach_host_self();
        let mut stats: libc::vm_statistics64 = std::mem::zeroed();
        let mut count = libc::HOST_VM_INFO64_COUNT;
        let status = libc::host_statistics64(
            host,
            libc::HOST_VM_INFO64,
            (&mut stats as *mut libc::vm_statistics64).cast(),
            &mut count,
        );
        mach_port_deallocate(libc::mach_task_self(), host);
        let page = libc::sysconf(libc::_SC_PAGESIZE);
        if status != libc::KERN_SUCCESS || page <= 0 {
            return None;
        }
        // free_count already includes speculative pages. Inactive pages supply
        // a reclaimable-memory estimate; do not double-count speculative pages.
        let pages = u64::from(stats.free_count) + u64::from(stats.inactive_count);
        checked(total, pages.checked_mul(page as u64)?)
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
pub fn available_memory() -> Option<MemorySnapshot> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogue_capacity_tracks_memory_without_removing_the_guard() {
        let ample = Some(MemorySnapshot {
            total: 32 * GIB,
            available: 9 * GIB,
        });
        assert_eq!(catalogue_limit(None, ample), 4_194_304);
        let constrained = Some(MemorySnapshot {
            total: 8 * GIB,
            available: 2 * GIB,
        });
        assert_eq!(catalogue_limit(None, constrained), 524_288);
        assert_eq!(catalogue_limit(None, None), 1_000_000);
        assert_eq!(catalogue_limit(Some(3_000_000), constrained), 3_000_000);
        assert_eq!(
            catalogue_limit(
                None,
                Some(MemorySnapshot {
                    total: GIB,
                    available: 0
                })
            ),
            1
        );
        assert!(validate_catalogue_limit(Some(0)).is_err());
        assert!(validate_catalogue_limit(Some(100_000_001)).is_err());
    }

    #[test]
    fn automatic_budget_keeps_headroom_and_ignores_its_own_allocation() {
        let memory = MemorySnapshot {
            total: 32 * GIB,
            available: 9 * GIB,
        };
        let initial = budget(None, Some(memory), 0);
        assert_eq!(initial.bytes, 2 * GIB);
        let after = MemorySnapshot {
            available: 8 * GIB,
            ..memory
        };
        assert_eq!(budget(None, Some(after), GIB).bytes, initial.bytes);
        let pressure = MemorySnapshot {
            available: 256 * MIB,
            ..memory
        };
        assert_eq!(budget(None, Some(pressure), 0).bytes, 0);
        assert_eq!(
            budget(
                None,
                Some(MemorySnapshot {
                    total: 128 * GIB,
                    available: 120 * GIB
                }),
                0
            )
            .bytes,
            8 * GIB
        );
        assert_eq!(budget(Some(512), Some(pressure), 0).bytes, 512 * MIB);
        assert_eq!(budget(None, None, 0).bytes, 256 * MIB);
    }

    #[test]
    fn memory_parser_requires_available_memory_and_checked_units() {
        assert_eq!(
            linux_memory("MemTotal: 8388608 kB\nMemAvailable: 4194304 kB\n"),
            Some(MemorySnapshot {
                total: 8 * GIB,
                available: 4 * GIB
            })
        );
        assert!(linux_memory("MemTotal: 1024 kB\nMemFree: 128 kB\n").is_none());
        assert!(linux_memory("MemTotal: 1024 bytes\nMemAvailable: 128 kB\n").is_none());
        assert!(!should_resize(GIB, GIB + MIB));
        assert!(should_resize(GIB, 512 * MIB));
        assert!(should_resize(MIB, 0));
    }
}
