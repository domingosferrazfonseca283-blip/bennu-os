use super::{CellAction, CellId, CellState, MAX_CELLS};
use crate::arch::x86_64::execution;
use crate::arch::x86_64::pit;
use core::sync::atomic::{AtomicU32, Ordering};

static CURRENT_CELL: AtomicU32 = AtomicU32::new(u32::MAX);
static SCHEDULER_EPOCH: AtomicU32 = AtomicU32::new(0);

pub fn current_cell() -> Option<CellId> {
    let id = CURRENT_CELL.load(Ordering::Acquire);
    if id == u32::MAX { None } else { Some(CellId(id)) }
}

pub fn scheduler_epoch() -> u32 {
    SCHEDULER_EPOCH.load(Ordering::Acquire)
}

pub fn clear_current_cell() {
    CURRENT_CELL.store(u32::MAX, Ordering::Release);
}

/// Entry trampoline for native Bennu Cell execution.
///
/// A Cell owns its stack and returns to the scheduler only through the
/// execution fabric. The trampoline is deliberately small: obtain the entry,
/// execute it, publish the resulting state, then yield the CPU stack.
pub extern "C" fn cell_trampoline() -> ! {
    unsafe {
        if crate::memory::paging::switch_address_space(crate::memory::paging::kernel_root()).is_err() {
            loop { core::hint::spin_loop(); }
        }
    }

    loop {
        let id = CellId(CURRENT_CELL.load(Ordering::Acquire));
        if let Some((rip, rsp)) = super::runtime::user_entry(id) {
            unsafe { crate::arch::x86_64::userspace::enter(rip, rsp); }
        }
        let entry = match super::runtime::entry(id) {
            Some(entry) => entry,
            None => {
                let _ = super::runtime::finish_cell(id, CellAction::Stop);
                unsafe { let _ = execution::switch_back_to_scheduler(); }
                continue;
            }
        };

        let action = entry();
        let _ = super::runtime::finish_cell(id, action);

        unsafe {
            let _ = execution::switch_back_to_scheduler();
        }
    }
}

pub struct Scheduler {
    cursor: usize,
}

impl Scheduler {
    pub const fn new() -> Self {
        Self { cursor: 0 }
    }

    pub fn step(&mut self) -> Option<CellId> {
        if !pit::take_tick() {
            return None;
        }

        SCHEDULER_EPOCH.fetch_add(1, Ordering::AcqRel);

        for offset in 0..MAX_CELLS {
            let index = (self.cursor + offset) % MAX_CELLS;
            let id = CellId(index as u32);

            if super::runtime::cell_state(id) != Some(CellState::Ready) {
                continue;
            }

            let (context, address_space_root) = match super::runtime::context_ptr(id) {
                Some(context) => context,
                None => continue,
            };

            self.cursor = (index + 1) % MAX_CELLS;
            CURRENT_CELL.store(id.0, Ordering::Release);

            if super::runtime::start_cell(id).is_err() {
                // The identity is only valid while this Cell is actually
                // executing. Never leave a failed dispatch as current.
                clear_current_cell();
                continue;
            }

            unsafe {
                if execution::switch_to_cell(context, address_space_root).is_ok() {
                    // Returning here means the Cell yielded/stopped and the
                    // scheduler stack is active again.
                    clear_current_cell();
                    return Some(id);
                }
            }

            let _ = super::runtime::finish_cell(id, CellAction::Yield);
            clear_current_cell();
        }

        None
    }
}
