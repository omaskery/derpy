
#[derive(Copy, Clone, Debug)]
pub enum Verbosity {
    None,
    Info,
    Verbose,
}

pub struct Log {
    verbosity: Verbosity,
}

impl From<u64> for Log {
    fn from(other: u64) -> Log {
        Log {
            verbosity: match other {
                0 => Verbosity::None,
                1 => Verbosity::Info,
                2 => Verbosity::Verbose,
                _ => Verbosity::Verbose,
            },
        }
    }
}

impl Log {
    pub fn log(&self, verbosity: Verbosity, text: String) {
        if self.verbosity as usize >= verbosity as usize {
            println!("{}", text);
        }
    }

    pub fn verbose(&self, text: String) {
        self.log(Verbosity::Verbose, text)
    }

    pub fn info(&self, text: String) {
        self.log(Verbosity::Info, text)
    }
}

