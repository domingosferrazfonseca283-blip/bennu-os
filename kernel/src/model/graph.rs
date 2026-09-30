use super::object::ObjectId;
use super::MAX_OBJECTS;

const MAX_RELATIONS: usize = 512;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RelationKind {
    Contains = 1,
    Uses = 2,
    DependsOn = 3,
    Emits = 4,
    Observes = 5,
    Backs = 6,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Relation {
    pub from: ObjectId,
    pub to: ObjectId,
    pub kind: RelationKind,
}

static mut RELATIONS: [Option<Relation>; MAX_RELATIONS] = [None; MAX_RELATIONS];

pub fn init() {
    unsafe { RELATIONS = [None; MAX_RELATIONS]; }
}

pub fn link(from: ObjectId, to: ObjectId, kind: RelationKind) -> Result<(), &'static str> {
    if from.is_null() || to.is_null() {
        return Err("null graph endpoint");
    }
    unsafe {
        for slot in RELATIONS.iter_mut() {
            if slot.is_none() {
                *slot = Some(Relation { from, to, kind });
                return Ok(());
            }
        }
    }
    Err("resource graph full")
}

pub fn has_link(from: ObjectId, to: ObjectId, kind: RelationKind) -> bool {
    unsafe {
        RELATIONS.iter().any(|r| {
            matches!(r, Some(Relation { from: a, to: b, kind: k }) if *a == from && *b == to && *k == kind)
        })
    }
}

pub fn count() -> usize {
    unsafe { RELATIONS.iter().filter(|r| r.is_some()).count() }
}

pub const fn object_capacity() -> usize { MAX_OBJECTS }
