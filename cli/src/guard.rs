use std::process::Child;

pub struct ProcessGuard {
    child: Option<Child>,
    name: String,
}

impl ProcessGuard {
    pub fn new(child: Child, name: impl Into<String>) -> Self {
        Self {
            child: Some(child),
            name: name.into(),
        }
    }

    pub fn wait(mut self) -> std::io::Result<std::process::ExitStatus> {
        self.child.take().unwrap().wait()
    }

    pub fn kill(&mut self) -> std::io::Result<()> {
        if let Some(child) = &mut self.child {
            child.kill()
        } else {
            Ok(())
        }
    }

    pub fn take(mut self) -> Child {
        self.child.take().unwrap()
    }

    pub fn id(&self) -> u32 {
        self.child.as_ref().map(|c| c.id()).unwrap_or(0)
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        let pid = self.id();
        if let Some(Err(e)) = self.child.take().map(|mut c| c.kill()) {
            eprintln!("Warning: Failed to kill {} (pid {}): {}", self.name, pid, e);
        }
    }
}
