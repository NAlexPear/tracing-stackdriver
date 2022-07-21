use std::{
    fmt::{Formatter, Write},
    io,
};

/// Utility newtype for converting between fmt::Write and io::Write
// https://docs.rs/tracing-subscriber/latest/src/tracing_subscriber/fmt/writer.rs.html
pub(crate) struct WriteAdaptor<'a> {
    fmt_write: &'a mut dyn Write,
}

impl<'a> WriteAdaptor<'a> {
    pub(crate) fn new(fmt_write: &'a mut dyn Write) -> Self {
        Self { fmt_write }
    }
}

impl<'a> io::Write for WriteAdaptor<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let s =
            std::str::from_utf8(buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        self.fmt_write
            .write_str(s)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        Ok(s.as_bytes().len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'a> std::fmt::Debug for WriteAdaptor<'a> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.pad("WriteAdaptor { .. }")
    }
}
