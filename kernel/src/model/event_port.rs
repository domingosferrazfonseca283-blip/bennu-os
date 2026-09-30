use super::{Event, EventKind, ObjectId};

pub const PORT_CAPACITY: usize = 64;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct EventFilter {
    pub kind: EventKind,
    pub source: ObjectId,
    pub target: ObjectId,
}

impl EventFilter {
    pub const ANY: Self = Self {
        kind: EventKind::None,
        source: ObjectId::NULL,
        target: ObjectId::NULL,
    };

    pub fn matches(&self, event: Event) -> bool {
        (self.kind == EventKind::None || self.kind == event.kind)
            && (self.source.is_null() || self.source == event.source)
            && (self.target.is_null() || self.target == event.target)
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct EventPort {
    pub object: ObjectId,
    pub filter: EventFilter,
    pub queue: [Event; PORT_CAPACITY],
    pub head: usize,
    pub tail: usize,
}

impl EventPort {
    pub const EMPTY: Self = Self {
        object: ObjectId::NULL,
        filter: EventFilter::ANY,
        queue: [Event::EMPTY; PORT_CAPACITY],
        head: 0,
        tail: 0,
    };

    pub fn push(&mut self, event: Event) -> Result<(), &'static str> {
        if !self.filter.matches(event) {
            return Err("event rejected by port filter");
        }
        let next = (self.tail + 1) % PORT_CAPACITY;
        if next == self.head {
            return Err("event port full");
        }
        self.queue[self.tail] = event;
        self.tail = next;
        Ok(())
    }

    pub fn pop(&mut self) -> Option<Event> {
        if self.head == self.tail {
            return None;
        }
        let event = self.queue[self.head];
        self.head = (self.head + 1) % PORT_CAPACITY;
        Some(event)
    }

    pub fn set_filter(&mut self, filter: EventFilter) {
        self.filter = filter;
    }

    pub const fn is_empty(&self) -> bool {
        self.head == self.tail
    }
}
