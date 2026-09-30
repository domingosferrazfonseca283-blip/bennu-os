use super::object::ObjectId;
use super::sync::SpinLock;
pub const MAX_SURFACES: usize = 16;
#[repr(C)] #[derive(Clone,Copy)] pub struct Surface{pub object:ObjectId,pub width:u32,pub height:u32,pub stride:u32,pub format:u32,pub buffer:u64}
impl Surface{pub const EMPTY:Self=Self{object:ObjectId::NULL,width:0,height:0,stride:0,format:0,buffer:0};}
static SURFACES:SpinLock<[Surface;MAX_SURFACES]>=SpinLock::new([Surface::EMPTY;MAX_SURFACES]);
pub fn init(){*SURFACES.lock().get_mut()=[Surface::EMPTY;MAX_SURFACES];}
pub fn attach(object:ObjectId,width:u32,height:u32,buffer:u64)->Result<usize,&'static str>{
 if object.is_null()||width==0||height==0{return Err("invalid surface");}
 let mut g=SURFACES.lock();for(i,s)in g.get_mut().iter_mut().enumerate(){if s.object.is_null(){*s=Surface{object,width,height,stride:width,format:1,buffer};return Ok(i);}}Err("surface table full")
}
pub fn get(index:usize)->Option<Surface>{if index>=MAX_SURFACES{return None;}let g=SURFACES.lock();let s=g.get()[index];if s.object.is_null(){None}else{Some(s)}}
