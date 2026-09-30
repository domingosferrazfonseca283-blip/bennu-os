use super::{CellAction, CellId, CellState, MAX_CELLS};
use crate::arch::x86_64::execution;
use crate::arch::x86_64::pit;
use core::sync::atomic::{AtomicU32, Ordering};

static CURRENT_CELL: AtomicU32 = AtomicU32::new(u32::MAX);

/// Entry trampoline for native Bennu Cell execution.
///
/// A Cell owns its stack and returns to the scheduler only through the
/// execution fabric. The trampoline is deliberately small: obtain the entry,
/// execute it, publish the resulting state, then yield the CPU stack.
pub extern "C" fn cell_trampoline() -> ! {
    loop {
        let id = CellId(CURRENT_CELL.load(Ordering::Acquire));
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

        for offset in 0..MAX_CELLS {
            let index = (self.cursor + offset) % MAX_CELLS;
            let id = CellId(index as u32);

            if super::runtime::cell_state(id) != Some(CellState::Ready) {
                continue;
            }

            let context = match super::runtime::context_ptr(id) {
                Some(context) => context,
                None => continue,
            };

            self.cursor = (index + 1) % MAX_CELLS;
            CURRENT_CELL.store(id.0, Ordering::Release);

            unsafe {
                if execution::switch_to_cell(context).is_ok() {
                    return Some(id);
                }
            }
        }

        None
    }
}
