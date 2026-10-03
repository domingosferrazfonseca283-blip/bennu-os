use super::boot_info::{BootInfo, BENNU_FRAMEBUFFER_BYTES_PER_PIXEL};
use crate::model::compositor::CompositionCommand;
use crate::model::graphics::PixelFormat;

pub struct Framebuffer {
    base: *mut u32, width: usize, height: usize, pitch_pixels: usize,
    red_mask: u8, red_shift: u8, green_mask: u8, green_shift: u8,
    blue_mask: u8, blue_shift: u8,
}
impl Framebuffer {
    fn validate(b:&BootInfo)->Result<(),&'static str>{
        if b.framebuffer_addr==0||b.framebuffer_width==0||b.framebuffer_height==0||b.framebuffer_pitch==0||b.framebuffer_bpp!=32||b.framebuffer_pitch%BENNU_FRAMEBUFFER_BYTES_PER_PIXEL!=0||b.framebuffer_addr&3!=0{return Err("no supported framebuffer");}
        let min=(b.framebuffer_width as u64).checked_mul(BENNU_FRAMEBUFFER_BYTES_PER_PIXEL as u64).ok_or("framebuffer pitch overflow")?;
        if (b.framebuffer_pitch as u64)<min{return Err("framebuffer pitch is too small");}
        if b.framebuffer_red_position>=32||b.framebuffer_green_position>=32||b.framebuffer_blue_position>=32||(b.framebuffer_red_position as u16+b.framebuffer_red_mask as u16)>32||(b.framebuffer_green_position as u16+b.framebuffer_green_mask as u16)>32||(b.framebuffer_blue_position as u16+b.framebuffer_blue_mask as u16)>32{return Err("invalid framebuffer channel layout");} Ok(())
    }
    pub unsafe fn from_boot_info(b:&BootInfo)->Result<Self,&'static str>{Self::validate(b)?;let bytes=(b.framebuffer_pitch as u64).checked_mul(b.framebuffer_height as u64).ok_or("framebuffer size overflow")?;let base=crate::memory::paging::map_mmio(b.framebuffer_addr,bytes)? as *mut u32;Ok(Self{base,width:b.framebuffer_width as usize,height:b.framebuffer_height as usize,pitch_pixels:b.framebuffer_pitch as usize/4,red_mask:b.framebuffer_red_mask,red_shift:b.framebuffer_red_position,green_mask:b.framebuffer_green_mask,green_shift:b.framebuffer_green_position,blue_mask:b.framebuffer_blue_mask,blue_shift:b.framebuffer_blue_position})}
    fn pixel(&self,r:u8,g:u8,b:u8)->u32{(((r as u32*self.red_mask as u32+127)/255)<<self.red_shift)|(((g as u32*self.green_mask as u32+127)/255)<<self.green_shift)|(((b as u32*self.blue_mask as u32+127)/255)<<self.blue_shift)}
    pub unsafe fn clear(&self,p:u32){for y in 0..self.height{let row=self.base.add(y*self.pitch_pixels);for x in 0..self.width{core::ptr::write_volatile(row.add(x),p);}}}
    pub unsafe fn fill_rect(&self,x:usize,y:usize,w:usize,h:usize,p:u32){let x2=core::cmp::min(x.saturating_add(w),self.width);let y2=core::cmp::min(y.saturating_add(h),self.height);for yy in y..y2{let row=self.base.add(yy*self.pitch_pixels);for xx in x..x2{core::ptr::write_volatile(row.add(xx),p);}}}
    unsafe fn draw_command(&self,c:&CompositionCommand)->Result<(),&'static str>{
        let bpp=match c.format{PixelFormat::Rgba8888|PixelFormat::Bgra8888=>4usize,PixelFormat::Unknown=>return Err("unsupported surface format")};
        if c.buffer==0||c.stride<c.width.saturating_mul(bpp as u32){return Err("invalid composition buffer");}
        let source_end=(c.source_y as u64).checked_mul(c.stride as u64).and_then(|v|v.checked_add(c.source_x as u64*bpp as u64)).and_then(|v|v.checked_add((c.height.saturating_sub(1) as u64)*c.stride as u64)).and_then(|v|v.checked_add(c.width as u64*bpp as u64)).ok_or("composition buffer overflow")?;
        let src=c.buffer as *const u8;
        for row in 0..c.height{
            let off=(c.source_y as u64+row as u64).checked_mul(c.stride as u64).and_then(|v|v.checked_add(c.source_x as u64*bpp as u64)).ok_or("composition row overflow")? as usize;
            let dy=c.destination_y as usize+row as usize;if dy>=self.height{break;}
            let width=core::cmp::min(c.width as usize,self.width.saturating_sub(c.destination_x as usize));
            let source=src.add(off);let dest=self.base.add(dy*self.pitch_pixels+c.destination_x as usize);
            for col in 0..width{let value=core::ptr::read_unaligned(source.add(col*bpp) as *const u32);let value=match c.format{PixelFormat::Rgba8888=>value,PixelFormat::Bgra8888=>{let r=value&0xff;let g=value&0xff00;let b=value&0xff0000;let a=value&0xff000000;(b<<16)|g|(r>>16)|a},PixelFormat::Unknown=>return Err("unsupported surface format")};core::ptr::write_volatile(dest.add(col),value);}
        }
        let _=source_end;Ok(())
    }
    pub unsafe fn render(&self,commands:&[CompositionCommand]){for c in commands{if c.destination_x<self.width as u32&&c.destination_y<self.height as u32&&c.width!=0&&c.height!=0{let _=self.draw_command(c);}}}
}
fn glyph(byte:u8,row:usize)->u8{const F:[[u8;8];16]=[[0x3c,0x66,0x6e,0x76,0x66,0x66,0x3c,0],[0x18,0x38,0x18,0x18,0x18,0x18,0x7e,0],[0x3c,0x66,6,0x0c,0x30,0x60,0x7e,0],[0x3c,0x66,6,0x1c,6,0x66,0x3c,0],[0x0c,0x1c,0x3c,0x6c,0x7e,0x0c,0x0c,0],[0x7e,0x60,0x7c,6,6,0x66,0x3c,0],[0x1c,0x30,0x60,0x7c,0x66,0x66,0x3c,0],[0x7e,6,0x0c,0x18,0x30,0x30,0x30,0],[0x3c,0x66,0x66,0x3c,0x66,0x66,0x3c,0],[0x3c,0x66,0x66,0x3e,6,0x0c,0x38,0],[0,0,0,0,0,0,0,0],[0x18,0x3c,0x66,0x66,0x7e,0x66,0x66,0],[0x7c,0x66,0x66,0x7c,0x66,0x66,0x7c,0],[0x3c,0x66,0x60,0x60,0x60,0x66,0x3c,0],[0x78,0x6c,0x66,0x66,0x66,0x6c,0x78,0],[0x7e,0x60,0x60,0x7c,0x60,0x60,0x7e,0]];let i=match byte{b'A'..=b'F'=>(byte-b'A')as usize+10,b'0'..=b'9'=>(byte-b'0')as usize,b' '=>10,_=>10};F[i][row]}
impl Framebuffer{pub unsafe fn text(&self,x:usize,y:usize,text:&[u8],scale:usize,fg:u32){let s=core::cmp::max(scale,1);let mut c=x;let mut base=y;for &byte in text{if byte==b'\n'{c=x;base=base.saturating_add(9*s);continue;}if c.saturating_add(8*s)>self.width{c=x;base=base.saturating_add(9*s);}if base.saturating_add(8*s)>self.height{break;}for row in 0..8{let bits=glyph(byte,row);for col in 0..8{if bits&(0x80>>col)!=0{self.fill_rect(c+col*s,base+row*s,s,s,fg);}}}c=c.saturating_add(8*s);}}}
pub fn init(b:&BootInfo)->Result<(),&'static str>{let fb=unsafe{Framebuffer::from_boot_info(b)?};let a=fb.pixel(16,24,40);let d=fb.pixel(32,48,72);let black=fb.pixel(0,0,0);unsafe{for y in 0..fb.height{let row=fb.base.add(y*fb.pitch_pixels);for x in 0..fb.width{core::ptr::write_volatile(row.add(x),if((x/64+y/64)&1)==0{a}else{d});}}let bar=core::cmp::min(72,fb.height);fb.fill_rect(0,0,fb.width,bar,black);let white=fb.pixel(255,255,255);let accent=fb.pixel(64,160,255);let text=fb.pixel(160,216,255);fb.text(24,20,b"BENNU OS",2,white);fb.fill_rect(24,112,core::cmp::min(360,fb.width.saturating_sub(48)),2,accent);fb.text(24,136,b"VIDEO ONLINE",2,text);}Ok(())}
pub fn present(b:&BootInfo,commands:&[CompositionCommand])->Result<(),&'static str>{let fb=unsafe{Framebuffer::from_boot_info(b)?};unsafe{fb.render(commands);}Ok(())}
