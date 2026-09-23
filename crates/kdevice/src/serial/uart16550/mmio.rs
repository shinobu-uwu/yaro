use crate::serial::uart16550::Backend;

#[derive(Debug)]
pub struct Mmio(usize);

impl Backend for Mmio {
    fn read(&self, offset: u8) -> u8 {
        todo!()
    }

    fn write(&mut self, offset: u8, value: u8) {
        todo!()
    }
}
