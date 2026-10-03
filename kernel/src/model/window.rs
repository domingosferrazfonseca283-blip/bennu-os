use super::object::ObjectId;
use super::sync::SpinLock;

pub const MAX_WINDOWS: usize = 32;

#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct WindowId(pub u16);

impl WindowId {
    pub const NULL: Self = Self(u16::MAX);

    pub const fn is_null(self) -> bool {
        self.0 == u16::MAX
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Window {
    pub id: WindowId,
    pub owner: ObjectId,
    pub surface: ObjectId,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub flags: u32,
}

impl Window {
    pub const EMPTY: Self = Self {
        id: WindowId::NULL,
        owner: ObjectId::NULL,
        surface: ObjectId::NULL,
        x: 0,
        y: 0,
        width: 0,
        height: 0,
        flags: 0,
    };

    pub const fn valid(&self) -> bool {
        !self.id.is_null()
            && !self.owner.is_null()
            && !self.surface.is_null()
            && self.width != 0
            && self.height != 0
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        if !self.valid() || x < self.x || y < self.y {
            return false;
        }
        let right = match self.x.checked_add(self.width as i32) {
            Some(value) => value,
            None => return false,
        };
        let bottom = match self.y.checked_add(self.height as i32) {
            Some(value) => value,
            None => return false,
        };
        x < right && y < bottom
    }
}

#[derive(Clone, Copy)]
struct WindowStack {
    windows: [Window; MAX_WINDOWS],
    order: [WindowId; MAX_WINDOWS],
    count: usize,
    focused: WindowId,
}

impl WindowStack {
    const EMPTY: Self = Self {
        windows: [Window::EMPTY; MAX_WINDOWS],
        order: [WindowId::NULL; MAX_WINDOWS],
        count: 0,
        focused: WindowId::NULL,
    };

    fn slot(&self, id: WindowId) -> Option<usize> {
        let mut i = 0;
        while i < self.count {
            if self.order[i] == id {
                return Some(i);
            }
            i += 1;
        }
        None
    }

    fn window_index(&self, id: WindowId) -> Option<usize> {
        let index = id.0 as usize;
        if index >= MAX_WINDOWS {
            return None;
        }
        if self.windows[index].id == id {
            Some(index)
        } else {
            None
        }
    }

    fn allocate_id(&self) -> Option<WindowId> {
        let mut i = 0;
        while i < MAX_WINDOWS {
            if self.windows[i].id.is_null() {
                return Some(WindowId(i as u16));
            }
            i += 1;
        }
        None
    }

    fn bring_to_front(&mut self, id: WindowId) -> Result<(), &'static str> {
        let position = self.slot(id).ok_or("window is not stacked")?;
        let mut i = position;
        while i + 1 < self.count {
            self.order[i] = self.order[i + 1];
            i += 1;
        }
        self.order[self.count - 1] = id;
        self.focused = id;
        Ok(())
    }

    fn send_to_back(&mut self, id: WindowId) -> Result<(), &'static str> {
        let position = self.slot(id).ok_or("window is not stacked")?;
        if position != 0 {
            let mut i = position;
            while i > 0 {
                self.order[i] = self.order[i - 1];
                i -= 1;
            }
            self.order[0] = id;
        }
        if self.focused == id {
            self.focused = if self.count > 1 {
                self.order[self.count - 1]
            } else {
                WindowId::NULL
            };
        }
        Ok(())
    }

    fn close(&mut self, id: WindowId) -> Result<(), &'static str> {
        let position = self.slot(id).ok_or("window is not stacked")?;
        let index = self.window_index(id).ok_or("window is not allocated")?;
        let was_focused = self.focused == id;

        let mut i = position;
        while i + 1 < self.count {
            self.order[i] = self.order[i + 1];
            i += 1;
        }
        self.order[self.count - 1] = WindowId::NULL;
        self.count -= 1;
        self.windows[index] = Window::EMPTY;

        if was_focused {
            self.focused = if self.count == 0 {
                WindowId::NULL
            } else {
                self.order[self.count - 1]
            };
        }
        Ok(())
    }
}

static WINDOWS: SpinLock<WindowStack> = SpinLock::new(WindowStack::EMPTY);

pub fn init() {
    *WINDOWS.lock().get_mut() = WindowStack::EMPTY;
}

pub fn create(
    owner: ObjectId,
    surface: ObjectId,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> Result<WindowId, &'static str> {
    if owner.is_null() || surface.is_null() || width == 0 || height == 0 {
        return Err("invalid window");
    }

    let mut stack = WINDOWS.lock();
    let state = stack.get_mut();
    if state.count == MAX_WINDOWS {
        return Err("window stack full");
    }

    let id = state.allocate_id().ok_or("window ids exhausted")?;
    let index = id.0 as usize;

    state.windows[index] = Window {
        id,
        owner,
        surface,
        x,
        y,
        width,
        height,
        flags: 0,
    };
    state.order[state.count] = id;
    state.count += 1;
    state.focused = id;
    super::compositor::mark_dirty();
    Ok(id)
}

pub fn bring_to_front(id: WindowId) -> Result<(), &'static str> {
    let result = WINDOWS.lock().get_mut().bring_to_front(id);
    if result.is_ok() { super::compositor::mark_dirty(); }
    result
}

pub fn send_to_back(id: WindowId) -> Result<(), &'static str> {
    let result = WINDOWS.lock().get_mut().send_to_back(id);
    if result.is_ok() { super::compositor::mark_dirty(); }
    result
}

pub fn close(id: WindowId) -> Result<(), &'static str> {
    let result = WINDOWS.lock().get_mut().close(id);
    if result.is_ok() { super::compositor::mark_dirty(); }
    result
}

pub fn front() -> Option<WindowId> {
    let stack = WINDOWS.lock();
    let state = stack.get();
    if state.count == 0 {
        None
    } else {
        Some(state.order[state.count - 1])
    }
}

pub fn focused() -> Option<WindowId> {
    let stack = WINDOWS.lock();
    let id = stack.get().focused;
    if id.is_null() {
        None
    } else {
        Some(id)
    }
}

pub fn focus(id: WindowId) -> Result<(), &'static str> {
    let result = WINDOWS.lock().get_mut().bring_to_front(id);
    if result.is_ok() { super::compositor::mark_dirty(); }
    result
}

pub fn hit_test(x: i32, y: i32) -> Option<WindowId> {
    let stack = WINDOWS.lock();
    let state = stack.get();
    let mut index = state.count;

    while index > 0 {
        index -= 1;
        let id = state.order[index];
        if let Some(window_index) = state.window_index(id) {
            if state.windows[window_index].contains(x, y) {
                return Some(id);
            }
        }
    }
    None
}

pub fn get(id: WindowId) -> Option<Window> {
    let stack = WINDOWS.lock();
    let state = stack.get();
    state.window_index(id).map(|index| state.windows[index])
}

pub fn order(out: &mut [WindowId]) -> usize {
    let stack = WINDOWS.lock();
    let state = stack.get();
    let count = core::cmp::min(out.len(), state.count);

    let mut i = 0;
    while i < count {
        out[i] = state.order[i];
        i += 1;
    }
    count
}
