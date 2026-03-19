use std::collections::VecDeque;
use std::env;
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;

#[derive(Debug, Clone, PartialEq, Eq)]
struct Config {
    root_dir: PathBuf,
    max_depth: usize,
    jobs: usize,
    command: Vec<OsString>,
}

#[derive(Debug)]
struct RunResult {
    target_dir: PathBuf,
    command_line: String,
    output: std::process::Output,
}

#[derive(Debug)]
enum AppError {
    Usage(String),
    Help(String),
    Io {
        action: &'static str,
        path: PathBuf,
        source: std::io::Error,
    },
    ParseInt {
        option: &'static str,
        value: String,
        source: std::num::ParseIntError,
    },
    CommandFailed {
        failed: usize,
        total: usize,
    },
    ThreadJoin,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage(message) => write!(f, "{message}"),
            Self::Help(message) => write!(f, "{message}"),
            Self::Io {
                action,
                path,
                source,
            } => write!(f, "{action} `{}` failed: {source}", path.display()),
            Self::ParseInt {
                option,
                value,
                source,
            } => write!(f, "invalid value `{value}` for {option}: {source}"),
            Self::CommandFailed { failed, total } => {
                write!(f, "{failed} command(s) failed across {total} directories")
            }
            Self::ThreadJoin => write!(f, "a worker thread panicked"),
        }
    }
}

impl std::error::Error for AppError {}

fn main() {
    match run() {
        Ok(()) => {}
        Err(AppError::Help(message)) => {
            println!("{message}");
        }
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
    }
}

fn run() -> Result<(), AppError> {
    let config = parse_args(env::args_os().skip(1))?;

    if !config.root_dir.is_dir() {
        return Err(AppError::Usage(format!(
            "`{}` is not a directory",
            config.root_dir.display()
        )));
    }

    let target_dirs = collect_target_dirs(&config.root_dir, config.max_depth)?;
    if target_dirs.is_empty() {
        println!(
            "No subdirectories found under `{}` within depth {}.",
            config.root_dir.display(),
            config.max_depth
        );
        return Ok(());
    }

    println!(
        "Executing `{}` in {} directories with max depth {} and {} worker(s).",
        format_command(&config.command),
        target_dirs.len(),
        config.max_depth,
        config.jobs
    );

    let results = execute_in_parallel(&target_dirs, &config.command, config.jobs)?;
    let failures = print_results(&results);

    if failures > 0 {
        return Err(AppError::CommandFailed {
            failed: failures,
            total: results.len(),
        });
    }

    Ok(())
}

fn parse_args<I>(args: I) -> Result<Config, AppError>
where
    I: IntoIterator<Item = OsString>,
{
    let mut args = args.into_iter().peekable();
    let mut max_depth = 1usize;
    let mut jobs = thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .max(1);
    let mut root_dir: Option<PathBuf> = None;
    let mut command = Vec::new();
    let mut after_separator = false;

    while let Some(arg) = args.next() {
        if after_separator {
            command.push(arg);
            continue;
        }

        match arg.to_string_lossy().as_ref() {
            "--" => {
                after_separator = true;
            }
            "-h" | "--help" => return Err(AppError::Help(usage())),
            "-d" | "--depth" => {
                let value = next_value(&mut args, "--depth")?;
                max_depth = parse_usize(value, "--depth")?;
            }
            "-j" | "--jobs" => {
                let value = next_value(&mut args, "--jobs")?;
                jobs = parse_usize(value, "--jobs")?.max(1);
            }
            _ if root_dir.is_none() => {
                root_dir = Some(PathBuf::from(arg));
            }
            _ => {
                command.push(arg);
                command.extend(args);
                break;
            }
        }
    }

    let root_dir = root_dir.ok_or_else(|| AppError::Usage(usage()))?;
    if command.is_empty() {
        return Err(AppError::Usage(usage()));
    }

    Ok(Config {
        root_dir,
        max_depth,
        jobs,
        command,
    })
}

fn next_value<I>(
    args: &mut std::iter::Peekable<I>,
    option: &'static str,
) -> Result<OsString, AppError>
where
    I: Iterator<Item = OsString>,
{
    args.next()
        .ok_or_else(|| AppError::Usage(format!("missing value for {option}\n\n{}", usage())))
}

fn parse_usize(value: OsString, option: &'static str) -> Result<usize, AppError> {
    let value_string = value.to_string_lossy().into_owned();
    let parsed = value_string
        .parse::<usize>()
        .map_err(|source| AppError::ParseInt {
            option,
            value: value_string.clone(),
            source,
        })?;

    Ok(parsed)
}

fn usage() -> String {
    [
        "Usage:",
        "  paracmd [--depth N] [--jobs N] <directory> -- <command> [args...]",
        "  paracmd [--depth N] [--jobs N] <directory> <command> [args...]",
        "",
        "Examples:",
        "  paracmd D:\\workspaces -- git checkout test",
        "  paracmd --depth 2 --jobs 8 ~/code cargo test",
    ]
    .join("\n")
}

fn collect_target_dirs(root_dir: &Path, max_depth: usize) -> Result<Vec<PathBuf>, AppError> {
    let mut queue = VecDeque::from([(root_dir.to_path_buf(), 0usize)]);
    let mut collected = Vec::new();

    while let Some((current_dir, depth)) = queue.pop_front() {
        if depth >= max_depth {
            continue;
        }

        let entries = fs::read_dir(&current_dir).map_err(|source| AppError::Io {
            action: "reading directory",
            path: current_dir.clone(),
            source,
        })?;

        for entry_result in entries {
            let entry = entry_result.map_err(|source| AppError::Io {
                action: "reading directory entry",
                path: current_dir.clone(),
                source,
            })?;
            let file_type = entry.file_type().map_err(|source| AppError::Io {
                action: "reading file type",
                path: entry.path(),
                source,
            })?;

            if !file_type.is_dir() {
                continue;
            }

            let path = entry.path();
            collected.push(path.clone());
            queue.push_back((path, depth + 1));
        }
    }

    collected.sort();
    Ok(collected)
}

fn execute_in_parallel(
    target_dirs: &[PathBuf],
    command: &[OsString],
    jobs: usize,
) -> Result<Vec<RunResult>, AppError> {
    let worker_count = jobs.max(1).min(target_dirs.len().max(1));
    let queue = Arc::new(Mutex::new(VecDeque::from(target_dirs.to_vec())));
    let (sender, receiver) = mpsc::channel();
    let shared_command = Arc::new(command.to_vec());
    let mut handles = Vec::with_capacity(worker_count);

    for _ in 0..worker_count {
        let queue = Arc::clone(&queue);
        let sender = sender.clone();
        let command = Arc::clone(&shared_command);

        handles.push(thread::spawn(move || {
            loop {
                let next_dir = {
                    let mut guard = queue.lock().expect("queue lock poisoned");
                    guard.pop_front()
                };

                let Some(target_dir) = next_dir else {
                    break;
                };

                let result = run_command(&target_dir, &command);
                if sender.send(result).is_err() {
                    break;
                }
            }
        }));
    }

    drop(sender);

    let mut results = Vec::with_capacity(target_dirs.len());
    for result in receiver {
        results.push(result);
    }

    for handle in handles {
        handle.join().map_err(|_| AppError::ThreadJoin)?;
    }

    results.sort_by(|left, right| left.target_dir.cmp(&right.target_dir));
    Ok(results)
}

fn run_command(target_dir: &Path, command: &[OsString]) -> RunResult {
    let command_line = format_command(command);
    let mut process = Command::new(&command[0]);
    process
        .args(&command[1..])
        .current_dir(target_dir)
        .stdin(std::process::Stdio::null());

    let output = match process.output() {
        Ok(output) => output,
        Err(error) => failed_output(error),
    };

    RunResult {
        target_dir: target_dir.to_path_buf(),
        command_line,
        output,
    }
}

#[cfg(unix)]
fn failed_output(error: std::io::Error) -> std::process::Output {
    use std::os::unix::process::ExitStatusExt;

    std::process::Output {
        status: ExitStatus::from_raw(1),
        stdout: Vec::new(),
        stderr: format!("failed to start command: {error}\n").into_bytes(),
    }
}

#[cfg(windows)]
fn failed_output(error: std::io::Error) -> std::process::Output {
    use std::os::windows::process::ExitStatusExt;

    std::process::Output {
        status: ExitStatus::from_raw(1),
        stdout: Vec::new(),
        stderr: format!("failed to start command: {error}\r\n").into_bytes(),
    }
}

fn print_results(results: &[RunResult]) -> usize {
    let mut failures = 0usize;

    for result in results {
        let status = if result.output.status.success() {
            "SUCCESS"
        } else {
            failures += 1;
            "FAILED"
        };

        println!(
            "[{status}] {} => {}",
            result.target_dir.display(),
            result.command_line
        );

        if !result.output.stdout.is_empty() {
            println!("stdout:");
            print!("{}", String::from_utf8_lossy(&result.output.stdout));
            if !result.output.stdout.ends_with(b"\n") {
                println!();
            }
        }

        if !result.output.stderr.is_empty() {
            eprintln!("stderr:");
            eprint!("{}", String::from_utf8_lossy(&result.output.stderr));
            if !result.output.stderr.ends_with(b"\n") {
                eprintln!();
            }
        }

        println!("exit: {}\n", exit_code_label(result.output.status));
    }

    println!(
        "Completed {} command(s): {} succeeded, {} failed.",
        results.len(),
        results.len().saturating_sub(failures),
        failures
    );

    failures
}

fn exit_code_label(status: ExitStatus) -> String {
    match status.code() {
        Some(code) => code.to_string(),
        None => "terminated by signal".to_string(),
    }
}

fn format_command(command: &[OsString]) -> String {
    command
        .iter()
        .map(shell_escape)
        .collect::<Vec<_>>()
        .join(" ")
}

fn shell_escape(value: &OsString) -> String {
    let value = value.to_string_lossy();
    if value.is_empty() {
        return "\"\"".to_string();
    }

    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | '\\' | ':'))
    {
        return value.into_owned();
    }

    format!("\"{}\"", value.replace('"', "\\\""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn parse_args_uses_defaults_without_separator() {
        let config = parse_args(vec![
            OsString::from("repos"),
            OsString::from("git"),
            OsString::from("checkout"),
            OsString::from("test"),
        ])
        .expect("config should parse");

        assert_eq!(config.root_dir, PathBuf::from("repos"));
        assert_eq!(config.max_depth, 1);
        assert!(!config.command.is_empty());
        assert_eq!(
            config.command,
            vec![
                OsString::from("git"),
                OsString::from("checkout"),
                OsString::from("test")
            ]
        );
    }

    #[test]
    fn parse_args_supports_separator_and_overrides() {
        let config = parse_args(vec![
            OsString::from("--depth"),
            OsString::from("2"),
            OsString::from("--jobs"),
            OsString::from("4"),
            OsString::from("repos"),
            OsString::from("--"),
            OsString::from("cargo"),
            OsString::from("test"),
        ])
        .expect("config should parse");

        assert_eq!(config.max_depth, 2);
        assert_eq!(config.jobs, 4);
        assert_eq!(config.root_dir, PathBuf::from("repos"));
        assert_eq!(
            config.command,
            vec![OsString::from("cargo"), OsString::from("test")]
        );
    }

    #[test]
    fn collect_target_dirs_respects_depth() {
        let root = unique_temp_dir();
        let first = root.join("repo-a");
        let second = root.join("repo-b");
        let nested = first.join("nested");

        fs::create_dir_all(&nested).expect("nested directories");
        fs::create_dir_all(&second).expect("second directory");

        let depth_one = collect_target_dirs(&root, 1).expect("scan depth one");
        assert_eq!(depth_one, vec![first.clone(), second.clone()]);

        let depth_two = collect_target_dirs(&root, 2).expect("scan depth two");
        assert_eq!(depth_two, vec![first, nested, second]);

        fs::remove_dir_all(&root).expect("cleanup temp dir");
    }

    fn unique_temp_dir() -> PathBuf {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before epoch")
            .as_nanos();
        let index = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = env::temp_dir().join(format!("paracmd-test-{seed}-{index}"));
        fs::create_dir_all(&path).expect("create temp dir");
        path
    }
}
