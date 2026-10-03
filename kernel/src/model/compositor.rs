use super::{graphics, surface, window, WindowId};
use core::sync::atomic::{AtomicBool, Ordering};

pub const MAX_COMPOSITION_WINDOWS: usize = window::MAX_WINDOWS;
pub const MAX_VISIBLE_RECTS: usize = MAX_COMPOSITION_WINDOWS * 4 + 4;

static COMPOSITION_DIRTY: AtomicBool = AtomicBool::new(false);

pub fn mark_dirty() {
    COMPOSITION_DIRTY.store(true, Ordering::Release);
}

pub fn take_dirty() -> bool {
    COMPOSITION_DIRTY.swap(false, Ordering::AcqRel)
}

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
    pub stride: u32,
    pub format: super::graphics::PixelFormat,
}

#[derive(Clone, Copy)]
struct Rect {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

impl Rect {
    const EMPTY: Self = Self { left: 0, top: 0, right: 0, bottom: 0 };

    const fn valid(&self) -> bool {
        self.right > self.left && self.bottom > self.top
    }

    const fn intersects(&self, other: &Self) -> bool {
        self.left < other.right
            && self.right > other.left
            && self.top < other.bottom
            && self.bottom > other.top
    }
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
        if width != 0 && height != 0 && !entry.surface.buffer.is_null() {
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
                out[written] = command_for_rect(entry, Rect {
                    left: left as i32,
                    top: top as i32,
                    right: right as i32,
                    bottom: bottom as i32,
                });
                written += 1;
            }
        }
        i += 1;
    }
    written
}

pub fn build_visible_commands(
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
    let mut written = 0;
    let mut index = count;

    while index > 0 {
        index -= 1;
        let entry = entries[index];
        if !entry.surface.valid() {
            continue;
        }

        let base = Rect {
            left: core::cmp::max(entry.x, 0),
            top: core::cmp::max(entry.y, 0),
            right: core::cmp::min(
                screen_width as i32,
                entry.x.saturating_add(entry.surface.width as i32),
            ),
            bottom: core::cmp::min(
                screen_height as i32,
                entry.y.saturating_add(entry.surface.height as i32),
            ),
        };
        if !base.valid() {
            continue;
        }

        let mut visible = [Rect::EMPTY; MAX_VISIBLE_RECTS];
        visible[0] = base;
        let mut visible_count = 1;
        let mut front = index + 1;

        while front < count && visible_count != 0 {
            let above = entries[front];
            if above.surface.valid() {
                let cover = Rect {
                    left: core::cmp::max(above.x, 0),
                    top: core::cmp::max(above.y, 0),
                    right: core::cmp::min(
                        screen_width as i32,
                        above.x.saturating_add(above.surface.width as i32),
                    ),
                    bottom: core::cmp::min(
                        screen_height as i32,
                        above.y.saturating_add(above.surface.height as i32),
                    ),
                };
                if cover.valid() {
                    visible_count = subtract_rects(&mut visible, visible_count, &cover);
                }
            }
            front += 1;
        }

        let mut piece = 0;
        while piece < visible_count && written < out.len() {
            out[written] = command_for_rect(entry, visible[piece]);
            written += 1;
            piece += 1;
        }
    }
    written
}

fn subtract_rects(rects: &mut [Rect], count: usize, cover: &Rect) -> usize {
    let mut index = 0;
    let mut current_count = count;

    while index < current_count {
        let rect = rects[index];
        if !rect.intersects(cover) {
            index += 1;
            continue;
        }

        let left = rect.left;
        let right = rect.right;
        let top = rect.top;
        let bottom = rect.bottom;
        current_count -= 1;
        rects[index] = rects[current_count];

        let pieces = [
            Rect {
                left,
                top,
                right,
                bottom: core::cmp::min(bottom, cover.top),
            },
            Rect {
                left,
                top: core::cmp::max(top, cover.bottom),
                right,
                bottom,
            },
            Rect {
                left,
                top: core::cmp::max(top, cover.top),
                right: core::cmp::min(right, cover.left),
                bottom: core::cmp::min(bottom, cover.bottom),
            },
            Rect {
                left: core::cmp::max(left, cover.right),
                top: core::cmp::max(top, cover.top),
                right,
                bottom: core::cmp::min(bottom, cover.bottom),
            },
        ];

        let mut p = 0;
        while p < pieces.len() {
            if pieces[p].valid() && current_count < rects.len() {
                rects[current_count] = pieces[p];
                current_count += 1;
            }
            p += 1;
        }
    }
    current_count
}

fn command_for_rect(entry: CompositionEntry, rect: Rect) -> CompositionCommand {
    let buffer = match graphics::get(entry.surface.buffer) {
        Some(value) => value,
        None => return CompositionCommand {
            window: WindowId::NULL,
            surface: super::object::ObjectId::NULL,
            source_x: 0,
            source_y: 0,
            destination_x: 0,
            destination_y: 0,
            width: 0,
            height: 0,
            buffer: 0,
            stride: 0,
            format: super::graphics::PixelFormat::Unknown,
        },
    };

    CompositionCommand {
        window: entry.window,
        surface: entry.surface.object,
        source_x: (rect.left - entry.x) as u32,
        source_y: (rect.top - entry.y) as u32,
        destination_x: rect.left as u32,
        destination_y: rect.top as u32,
        width: (rect.right - rect.left) as u32,
        height: (rect.bottom - rect.top) as u32,
        buffer: buffer.address,
        stride: buffer.stride,
        format: buffer.format,
    }
}

fn find_surface(object: super::object::ObjectId) -> Option<surface::Surface> {
    let mut index = 0;
    while index < surface::MAX_SURFACES {
        if let Some(value) = surface::get(index) {
            if value.object == object && graphics::get(value.buffer).is_some() {
                return Some(value);
            }
        }
        index += 1;
    }
    None
}
