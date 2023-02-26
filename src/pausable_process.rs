use winapi::um::debugapi;

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

    #[allow(unused)]
    pub fn is_finished(&mut self) -> bool {
        self.0.try_wait().unwrap().is_some()
    }

    pub fn pause(&mut self) -> Result<(), ()> {
        if self.1 == Status::Paused {
            return Ok(());
        }
        let out = match self.0.id() {
            Some(id) => unsafe { debugapi::DebugActiveProcess(id) },
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
            Some(id) => unsafe { debugapi::DebugActiveProcessStop(id) },
            _ => return Err(()),
        };
        if out != 0 {
            self.1 = Status::Working;
            Ok(())
        } else {
            Err(())
        }
    }
    #[allow(unused)]
    fn is_paused(&self) -> bool {
        self.1 == Status::Paused
    }
}
