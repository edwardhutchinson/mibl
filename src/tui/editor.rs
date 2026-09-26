//! Prepare an external editor command; the caller owns terminal suspension and waiting.
use mibl::model::Table;
use std::{
    path::Path,
    process::{Command, Stdio},
};

pub(super) fn command(directory: &Path, table: Table) -> Result<Command, String> {
    let editor = std::env::var("EDITOR").map_err(|error| match error {
        std::env::VarError::NotPresent => {
            "EDITOR is not set; set it to your editor command.".to_owned()
        }
        std::env::VarError::NotUnicode(_) => "EDITOR must be valid Unicode.".to_owned(),
    })?;
    let words = shell_words::split(&editor).map_err(|error| format!("invalid EDITOR: {error}"))?;
    let Some((program, arguments)) = words
        .split_first()
        .filter(|(program, _)| !program.is_empty())
    else {
        return Err("EDITOR is empty; set it to your editor command.".into());
    };
    let path = std::path::absolute(directory.join(table.file()))
        .map_err(|error| format!("cannot locate {}: {error}", table.file()))?;
    let metadata = std::fs::metadata(&path)
        .map_err(|error| format!("cannot open {} for editing: {error}", path.display()))?;
    if !metadata.is_file() {
        return Err(format!("{} is not a regular file", path.display()));
    }
    let mut command = Command::new(program);
    command
        .args(arguments)
        .arg(path)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    Ok(command)
}
