use super::{surface, window, WindowId};

pub const MAX_COMPOSITION_WINDOWS: usize = window::MAX_WINDOWS;

#[derive(Clone, Copy)]
pub struct CompositionEntry {
    pub window: WindowId,
    pub surface: super::surface::Surface,
}

pub fn collect(out: &mut [CompositionEntry]) -> usize {
    let mut ids = [WindowId::NULL; MAX_COMPOSITION_WINDOWS];
    let count = window::order(&mut ids);
    let limit = core::cmp::min(count, out.len());
    let mut written = 0;

    while written < limit {
        if let Some(win) = window::get(ids[written]) {
            if let Some(surface) = find_surface(win.surface) {
                out[written] = CompositionEntry {
                    window: win.id,
                    surface,
                };
                written += 1;
            }
        }
    }

    written
}

fn find_surface(object: super::object::ObjectId) -> Option<surface::Surface> {
    let mut index = 0;
    while index < surface::MAX_SURFACES {
        if let Some(value) = surface::get(index) {
            if value.object == object {
                return Some(value);
            }
        }
        index += 1;
    }
    None
}
