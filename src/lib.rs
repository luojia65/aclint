#![no_std]
#![feature(naked_functions, asm_const)]
#![deny(warnings)]

use core::{arch::asm, cell::UnsafeCell, mem::size_of};

/// Machine-level time counter register.
#[repr(transparent)]
pub struct MTIME(UnsafeCell<u64>);

/// Machine-level time compare register.
#[repr(transparent)]
pub struct MTIMECMP(UnsafeCell<u64>);

/// Machine-level IPI register.
#[repr(transparent)]
pub struct MSIP(UnsafeCell<u32>);

/// Set supervisor-level IPI register.
#[repr(transparent)]
pub struct SETSSIP(UnsafeCell<u32>);

/// Machine-level Software Interrupt Device (MSWI).
///
/// # Usage
///
/// ```no_run
/// impl rustsbi::Ipi for Clint {
///     #[inline]
///     fn send_ipi(&self, hart_mask: HartMask) -> SbiRet {
///         for i in hart_ids() {
///             if hart_mask.has_bit(i) && remote_hsm(i).map_or(false, |hsm| hsm.allow_ipi()) {
///                 // we assume this MSWI device covers hart id beginning from #0.
///                 self.mswi().set_msip(i);
///             }
///         }
///         SbiRet::success(0)
///     }
/// }
/// ```
#[repr(C)]
pub struct MSWI {
    /// HART index 0..4095 machine-level IPI registers.
    pub msip: [MSIP; 4095],
    _reserved: u32,
}

/// Supervisor-level Software Interrupt Device (SSWI).
#[repr(C)]
pub struct SSWI {
    pub setssip: [SETSSIP; 4095],
    _reserved: u32,
}

/// SiFive Core-Local Interruptor (CLINT) device.
#[repr(C)]
pub struct SifiveClint {
    pub mswi: MSWI,
    pub mtimecmp: [MTIMECMP; 4095],
    pub mtime: MTIME,
}

impl SifiveClint {
    const MTIMER_OFFSET: usize = size_of::<MSWI>() + size_of::<u32>();
    const MTIME_OFFSET: usize = Self::MTIMER_OFFSET + size_of::<[MTIMECMP; 4095]>();

    #[inline]
    pub fn read_mtime(&self) -> u64 {
        unsafe { self.mtime.0.get().read_volatile() }
    }

    #[inline]
    pub fn write_mtime(&self, val: u64) {
        unsafe { self.mtime.0.get().write_volatile(val) }
    }

    #[inline]
    pub fn read_mtimecmp(&self, hart_idx: usize) -> u64 {
        unsafe { self.mtimecmp[hart_idx].0.get().read_volatile() }
    }

    #[inline]
    pub fn write_mtimecmp(&self, hart_idx: usize, val: u64) {
        unsafe { self.mtimecmp[hart_idx].0.get().write_volatile(val) }
    }

    #[inline]
    pub fn read_msip(&self, hart_idx: usize) -> bool {
        unsafe { self.mswi.msip[hart_idx].0.get().read_volatile() != 0 }
    }

    #[inline]
    pub fn set_msip(&self, hart_idx: usize) {
        unsafe { self.mswi.msip[hart_idx].0.get().write_volatile(1) }
    }

    #[inline]
    pub fn clear_msip(&self, hart_idx: usize) {
        unsafe { self.mswi.msip[hart_idx].0.get().write_volatile(0) }
    }
}

impl SifiveClint {
    #[naked]
    pub extern "C" fn read_mtime_naked(&self) -> u64 {
        unsafe {
            asm!(
                "   addi sp, sp, -8
                    sd   a1, (sp)

                    li   a1, {offset}
                    add  a0, a0, a1

                    ld   a1, (sp)
                    addi sp, sp,  8

                    ld   a0, (a0)
                    ret
                ",
                offset = const Self::MTIME_OFFSET,
                options(noreturn),
            )
        }
    }

    #[naked]
    pub extern "C" fn write_mtime_naked(&self, val: u64) -> u64 {
        unsafe {
            asm!(
                "   addi sp, sp, -8
                    sd   a1, (sp)

                    li   a1, {offset}
                    add  a0, a0, a1

                    ld   a1, (sp)
                    addi sp, sp,  8

                    sd   a1, (a0)
                    ret
                ",
                offset = const Self::MTIME_OFFSET,
                options(noreturn),
            )
        }
    }

    #[naked]
    pub extern "C" fn read_mtimecmp_naked(&self, hart_idx: usize) -> u64 {
        unsafe {
            asm!(
                "   slli a1, a1, 3
                    add  a0, a0, a1

                    li   a1, {offset}
                    add  a0, a0, a1

                    ld   a0, (a0)
                    ret
                ",
                offset = const Self::MTIMER_OFFSET,
                options(noreturn),
            )
        }
    }

    #[naked]
    pub extern "C" fn write_mtimecmp_naked(&self, hart_idx: usize, val: u64) {
        unsafe {
            asm!(
                "   slli a1, a1, 3
                    add  a0, a0, a1

                    li   a1, {offset}
                    add  a0, a0, a1

                    sd   a2, (a0)
                    ret
                ",
                offset = const Self::MTIMER_OFFSET,
                options(noreturn),
            )
        }
    }

    #[naked]
    pub extern "C" fn read_msip_naked(&self, hart_idx: usize) -> bool {
        unsafe {
            asm!(
                "   slli a1, a1, 2
                    add  a0, a0, a1
                    lw   a0, (a0)
                    ret
                ",
                options(noreturn),
            )
        }
    }

    #[naked]
    pub extern "C" fn set_msip_naked(&self, hart_idx: usize) {
        unsafe {
            asm!(
                "   slli a1, a1, 2
                    add  a0, a0, a1
                    addi a1, zero, 1
                    sw   a1, (a0)
                    ret
                ",
                options(noreturn),
            )
        }
    }

    #[naked]
    pub extern "C" fn clear_msip_naked(&self, hart_idx: usize) {
        unsafe {
            asm!(
                "   slli a1, a1, 2
                    add  a0, a0, a1
                    sw   zero, (a0)
                    ret
                ",
                options(noreturn),
            )
        }
    }
}

#[test]
fn test() {
    assert_eq!(core::mem::size_of::<MSWI>(), 0x4000);
    assert_eq!(core::mem::size_of::<SSWI>(), 0x4000);
    assert_eq!(core::mem::size_of::<[MTIMECMP; 4095]>(), 0x7ff8);
    assert_eq!(core::mem::size_of::<SifiveClint>(), 0xc000);
}
