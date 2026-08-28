use std::{
    io::{self, Write},
    string::FromUtf8Error,
};

pub(crate) struct MarkdownPrinter {
    buf: Vec<u8>,
}

impl MarkdownPrinter {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    fn print(&mut self) -> Result<(), FromUtf8Error> {
        let output = String::from_utf8(std::mem::take(&mut self.buf))?;
        termimad::print_text(&output);
        Ok(())
    }
}

impl Write for MarkdownPrinter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buf.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.buf.flush()?;
        if !self.buf.is_empty() {
            self.print().map_err(io::Error::other)?;
        }
        Ok(())
    }
}

impl Drop for MarkdownPrinter {
    fn drop(&mut self) {
        if let Err(error) = self.flush() {
            log::error!("MarkdownPrinter.flush: {error}");
        }
    }
}
