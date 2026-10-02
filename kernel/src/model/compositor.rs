use super::{surface, window, WindowId};

pub const MAX_COMPOSITION_WINDOWS: usize = window::MAX_WINDOWS;

#[derive(Clone, Copy)]
pub struct CompositionEntry {
    pub window: WindowId,
    pub surface: surface::Surface,
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Copy)]
pub struct CompositionCommand {
    pub window: WindowId,
    pub surface: super::object::ObjectId,
    pub source_x: u32,
    pub source_y: u32,
    pub destination_x: u32,
    pub destination_y: u32,
    pub width: u32,
    pub height: u32,
    pub buffer: u64,
}

pub fn collect(out: &mut [CompositionEntry]) -> usize {
    let mut ids = [WindowId::NULL; MAX_COMPOSITION_WINDOWS];
    let count = window::order(&mut ids);
    let limit = core::cmp::min(count, out.len());
    let mut scanned = 0;
    let mut written = 0;

    while scanned < limit {
        let id = ids[scanned];
        if let Some(win) = window::get(id) {
            if let Some(value) = find_surface(win.surface) {
                out[written] = CompositionEntry {
                    window: win.id,
                    surface: value,
                    x: win.x,
                    y: win.y,
                };
                written += 1;
            }
        }
        scanned += 1;
    }

    written
}

pub fn build_commands(
    screen_width: u32,
    screen_height: u32,
    out: &mut [CompositionCommand],
) -> usize {
    let mut entries = [CompositionEntry {
        window: WindowId::NULL,
        surface: surface::Surface::EMPTY,
        x: 0,
        y: 0,
    }; MAX_COMPOSITION_WINDOWS];

    let count = collect(&mut entries);
    let limit = core::cmp::min(count, out.len());
    let mut i = 0;
    let mut written = 0;

    while i < limit {
        let entry = entries[i];
        let width = entry.surface.width;
        let height = entry.surface.height;

        if width != 0 && height != 0 && entry.surface.buffer != 0 {
            let left = core::cmp::max(entry.x as i64, 0);
            let top = core::cmp::max(entry.y as i64, 0);
            let right = core::cmp::min(
                screen_width as i64,
                entry.x as i64 + width as i64,
            );
            let bottom = core::cmp::min(
                screen_height as i64,
                entry.y as i64 + height as i64,
            );

            if right > left && bottom > top {
                let source_x = (left - entry.x as i64) as u32;
                let source_y = (top - entry.y as i64) as u32;

                out[written] = CompositionCommand {
                    window: entry.window,
                    surface: entry.surface.object,
                    source_x,
                    source_y,
                    destination_x: left as u32,
                    destination_y: top as u32,
                    width: (right - left) as u32,
                    height: (bottom - top) as u32,
                    buffer: entry.surface.buffer,
                };
                written += 1;
            }
        }

        i += 1;
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
