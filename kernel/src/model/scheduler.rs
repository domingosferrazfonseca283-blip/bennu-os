use super::{CellAction,CellId,CellState,MAX_CELLS};
use crate::arch::x86_64::pit;

pub struct Scheduler{cursor:usize}
impl Scheduler{
 pub const fn new()->Self{Self{cursor:0}}
 pub fn step(&mut self)->Option<CellId>{
  if !pit::take_tick(){return None;}
  for offset in 0..MAX_CELLS{
   let index=(self.cursor+offset)%MAX_CELLS;
   let id=CellId(index as u32);
   if super::runtime::cell_state(id)!=Some(CellState::Ready){continue;}
   self.cursor=(index+1)%MAX_CELLS;
   match super::runtime::run_once(id){
    Ok(CellAction::Yield)|Ok(CellAction::Wait)|Ok(CellAction::Stop)=>return Some(id),
    Err(_)=>{}
   }
  }
  None
 }
}
