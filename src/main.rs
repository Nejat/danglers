use std::env;
use std::fs::{create_dir_all, write};
use std::path::PathBuf;
use std::process::Command;

fn main() -> Result<(), String> {
    let project = env::args().skip(1).next().ok_or_else(|| String::from("Nothing to process"))?;
    let project_path = PathBuf::from(&project);

    if !project_path.exists() {
        return Err(format!(r#""{project}" does not exist"#));
    }

    let mut recovered = match project_path.file_name() {
        Some(project_name) => {
            let mut project_name = project_name.to_os_string();

            project_name.push("-danglers");

            PathBuf::from(project_name)
        },
        None => return Err(format!("could not extract project name from {project:?}"))
    };

    let git_output = Command::new("git")
        .current_dir(&project_path)
        .args(["fsck", "--no-reflog"])
        .output();

    let git_output = match git_output {
        Ok(git_output) => git_output,
        Err(err) => return Err(format!("git exception: {err}"))
    };

    let danglers = if git_output.status.success() {
        let stdout = String::from_utf8(git_output.stdout)
            .map_err(|err| format!("{err}"))?;

        stdout.split('\n')
            .filter_map(|ln| {
                if ln.starts_with("dangling") {
                    Some(String::from(ln.split(' ').last().unwrap()))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
    } else {
        eprintln!("STATUS: {}", git_output.status);

        match String::from_utf8(git_output.stderr) {
            Ok(stderr) => {
                eprintln!("FAILED: {stderr}");
            }
            Err(err) => {
                eprintln!("STDERR: {err}");
            }
        };

        return Err(String::from("git command failed to list danglers"));
    };

    create_dir_all(&recovered)
        .map_err(|err| format!("{err}"))?;

    recovered.push("place-holder-filename.ext");

    for dangler in danglers {
        let git_output = Command::new("git")
            .current_dir(&project_path)
            .args(["show", dangler.as_str()])
            .output();

        let git_output = match git_output {
            Ok(git_output) => git_output,
            Err(err) => {
                eprintln!("STDOUT: {dangler}: {err}");
                continue;
            }
        };

        if git_output.status.success() {
            match String::from_utf8(git_output.stdout) {
                Ok(content) => {
                    recovered.set_file_name(&dangler);
                    recovered.set_extension("txt");

                    if let Err(err) = write(&recovered, content) {
                        eprintln!("WRITE: {dangler}: {err}");
                    }
                }
                Err(err) => eprintln!("STDOUT: {dangler}: {err}")
            };
        } else {
            eprintln!("STATUS: {dangler}: {}", git_output.status);

            match String::from_utf8(git_output.stderr) {
                Ok(stderr) => {
                    eprintln!("FAILED: {stderr}");
                }
                Err(err) => {
                    eprintln!("STDERR: {err}");
                }
            };
        }
    }

    Ok(())
}
