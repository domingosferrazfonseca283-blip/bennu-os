use super::object::ObjectId;
use super::sync::SpinLock;
use super::MAX_OBJECTS;

const MAX_RELATIONS: usize = 512;
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RelationKind { Contains=1, Uses=2, DependsOn=3, Emits=4, Observes=5, Backs=6 }
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Relation { pub from:ObjectId, pub to:ObjectId, pub kind:RelationKind }
const EMPTY: Option<Relation> = None;
static RELATIONS: SpinLock<[Option<Relation>; MAX_RELATIONS]> = SpinLock::new([EMPTY; MAX_RELATIONS]);
pub fn init() { *RELATIONS.lock().get_mut() = [EMPTY; MAX_RELATIONS]; }
pub fn link(from:ObjectId,to:ObjectId,kind:RelationKind)->Result<(), &'static str>{
 if from.is_null()||to.is_null(){return Err("null graph endpoint");}
 let mut g=RELATIONS.lock(); for s in g.get_mut().iter_mut(){if s.is_none(){*s=Some(Relation{from,to,kind});return Ok(());}} Err("resource graph full")
}
pub fn has_link(from:ObjectId,to:ObjectId,kind:RelationKind)->bool{let g=RELATIONS.lock();g.get().iter().any(|r|matches!(r,Some(Relation{from:a,to:b,kind:k}) if *a==from&&*b==to&&*k==kind))}
pub fn count()->usize{let g=RELATIONS.lock();g.get().iter().filter(|r|r.is_some()).count()}
pub const fn object_capacity()->usize{MAX_OBJECTS}
