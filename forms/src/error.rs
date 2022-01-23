#[derive(Debug)]
pub enum Error {
    Windows(u32),
    ItemDeleted,
    ParentDeleted,
}

pub type Result<T> = core::result::Result<T, Error>;
