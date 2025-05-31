use std::io::{Write, stdin, stdout};

#[derive(Debug, Default)]
pub struct UserInput {
    prompt: String,
    buffer: String,
}

impl UserInput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_with_prompt(prompt: &str) -> Self {
        Self {
            prompt: prompt.into(),
            ..Default::default()
        }
    }
    pub fn set_prompt(&mut self, prompt: &str) {
        self.prompt = prompt.into();
    }

    pub fn read_line(&mut self) -> anyhow::Result<&str> {
        self.buffer.clear();
        print!("{}", self.prompt);
        let _ = stdout().flush();
        stdin().read_line(&mut self.buffer)?;
        Ok(self.buffer.trim())
    }
}
