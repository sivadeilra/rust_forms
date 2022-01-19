#[derive(Debug)]
pub enum Error {
    Windows(u32),
}

pub type Result<T> = core::result::Result<T, Error>;
