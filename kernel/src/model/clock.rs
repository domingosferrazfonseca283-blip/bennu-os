#[repr(C)]
#[derive(Clone,Copy)]
pub struct TimeSpec { pub seconds:u64,pub nanos:u32 }
impl TimeSpec { pub const ZERO:Self=Self{seconds:0,nanos:0}; }
#[repr(C)]
#[derive(Clone,Copy)]
pub struct Timer { pub deadline:u64,pub period:u64,pub target:u32 }
impl Timer { pub const EMPTY:Self=Self{deadline:0,period:0,target:0}; }
