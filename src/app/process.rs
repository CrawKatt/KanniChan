use std::path::Path;
use std::process::{Command, Stdio};

pub fn background_command(program: &Path) -> Command {
    let mut command = Command::new(program);

    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt as _;

        const CREATE_NO_WINDOW: u32 = 0x0800_0000;

        command.creation_flags(CREATE_NO_WINDOW);
    }

    command
}
