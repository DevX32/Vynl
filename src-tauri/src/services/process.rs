use std::process::Command as StdCommand;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[inline]
#[allow(unused_mut)]
pub fn hidden_std(mut cmd: StdCommand) -> StdCommand {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

#[inline]
#[allow(unused_mut)]
pub fn hidden_tokio(mut cmd: tokio::process::Command) -> tokio::process::Command {
    #[cfg(windows)]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}
