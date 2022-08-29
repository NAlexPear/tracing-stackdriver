use std::{
    io,
    sync::{Mutex, TryLockError},
};

pub struct MockWriter<'a>(pub &'a Mutex<Vec<u8>>);

impl<'a> MockWriter<'a> {
    pub fn map_err<G>(error: TryLockError<G>) -> io::Error {
        match error {
            TryLockError::WouldBlock => io::Error::from(io::ErrorKind::WouldBlock),
            TryLockError::Poisoned(_) => io::Error::from(io::ErrorKind::Other),
        }
    }
}

impl<'a> io::Write for MockWriter<'a> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.0.try_lock().map_err(Self::map_err)?.write(buffer)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.0.try_lock().map_err(Self::map_err)?.flush()
    }
}
