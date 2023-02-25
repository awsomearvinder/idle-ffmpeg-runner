#[derive(PartialEq, Eq, Debug)]
enum Status {
    Paused,
    Working,
}
pub struct PausableProcess(tokio::process::Child, Status);

impl PausableProcess {
    pub fn new(child: tokio::process::Child) -> Self {
        Self(child, Status::Working)
    }

    pub async fn wait(&mut self) -> Result<std::process::ExitStatus, std::io::Error> {
        self.0.wait().await
    }

    pub fn is_finished(&mut self) -> bool {
        self.0.try_wait().unwrap().is_some()
    }

    pub fn pause(&mut self) -> Result<(), ()> {
        if self.1 == Status::Paused {
            return Ok(());
        }
        let out = match self.0.id() {
            Some(id) => unsafe { winapi::um::debugapi::DebugActiveProcess(id) },
            _ => return Err(()),
        };
        if out != 0 {
            self.1 = Status::Paused;
            Ok(())
        } else {
            Err(())
        }
    }
    pub fn unpause(&mut self) -> Result<(), ()> {
        if self.1 != Status::Paused {
            return Ok(());
        }

        let out = match self.0.id() {
            Some(id) => unsafe { winapi::um::debugapi::DebugActiveProcessStop(id) },
            _ => return Err(()),
        };
        if out != 0 {
            self.1 = Status::Working;
            Ok(())
        } else {
            Err(())
        }
    }
    fn is_paused(&self) -> bool {
        self.1 == Status::Paused
    }
}

pub struct PauseOnDrop<'a>(&'a mut PausableProcess);
impl<'a> PauseOnDrop<'a> {
    pub fn new(i: &'a mut PausableProcess) -> PauseOnDrop<'a> {
        Self(i)
    }
    pub async fn wait(&mut self) -> Result<std::process::ExitStatus, std::io::Error> {
        self.0.wait().await
    }
}

impl<'a> Drop for PauseOnDrop<'a> {
    fn drop(&mut self) {
        if !self.0.is_finished() && !self.0.is_paused() {
            let _ = self.0.pause();
        }
    }
}
