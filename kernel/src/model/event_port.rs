use super::{Event,EventKind,ObjectId};

pub const PORT_CAPACITY:usize=64;

#[repr(C)]
#[derive(Clone,Copy)]
pub struct EventPort {
 pub object:ObjectId,
 pub queue:[Event;PORT_CAPACITY],
 pub head:usize,
 pub tail:usize,
}
impl EventPort {
 pub const EMPTY:Self=Self{object:ObjectId::NULL,queue:[Event::EMPTY;PORT_CAPACITY],head:0,tail:0};
 pub fn push(&mut self,event:Event)->Result<(),&'static str>{
  let next=(self.tail+1)%PORT_CAPACITY;if next==self.head{return Err("event port full");}
  self.queue[self.tail]=event;self.tail=next;Ok(())
 }
 pub fn pop(&mut self)->Option<Event>{
  if self.head==self.tail{return None;}let e=self.queue[self.head];self.head=(self.head+1)%PORT_CAPACITY;Some(e)
 }
 pub fn matches(&self,kind:EventKind,source:ObjectId)->bool{
  let _=(kind,source);true
 }
}
