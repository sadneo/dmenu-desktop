use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{env, fs};

use clap::{Parser, ValueEnum};
use ini::Ini;

#[derive(ValueEnum, Clone, Debug)]
enum EntryType {
    Name,
    Command,
    Filename,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[arg(long, default_value = "name")]
    entry_type: EntryType,

    /// Must point to a read-writeable file (will create if not exists). In this mode entries are sorted by usage frequency.
    #[arg(long)]
    usage_log: Option<PathBuf>,

    /// Determines the command used to invoke dmenu or an equivalent.
    #[arg(long)]
    dmenu: Option<String>,

    /// Terminal emulator used to launch applications, does nothing if dmenu is not provided, put {} where the dmenu command should go
    #[arg(long)]
    terminal: Option<String>,

    /// Which shell to execute commands with, defaults to $SHELL then /bin/sh
    #[arg(long)]
    shell: Option<String>,
}

#[derive(Debug)]
struct DesktopEntry {
    name: String,
    filename: String,
    exec: String,
    hide: bool,
    terminal: bool,
    path: Option<String>,
}

impl DesktopEntry {
    fn from_ini(filename: &str, ini: Ini) -> Option<DesktopEntry> {
        let section = ini.section(Some("Desktop Entry"))?;
        if section.get("Type") != Some("Application") {
            return None;
        }

        let name = section.get("Name")?;
        let exec = section.get("Exec")?;

        let try_exec = section.get("TryExec").map(str::to_string);
        let path = section.get("Path").map(str::to_string);
        let terminal = section.get("Terminal") == Some("true");

        let mut exec_exists = true;
        if let Some(try_exec) = &try_exec {
            exec_exists = PathBuf::from(try_exec).exists();
            if !exec_exists {
                exec_exists = exists_on_path(try_exec);
            }
        }

        let hide = !exec_exists
            || section.get("NoDisplay") == Some("true")
            || section.get("Hidden") == Some("true");

        Some(DesktopEntry {
            name: name.to_owned(),
            filename: filename.to_owned(),
            exec: exec.to_owned(),
            hide,
            terminal,
            path,
        })
    }
    fn field(&self, entry_type: &EntryType) -> &str {
        match entry_type {
            EntryType::Name => self.name.as_str(),
            EntryType::Filename => self.filename.as_str(),
            EntryType::Command => self.exec.split(" ").nth(0).unwrap_or(self.name.as_str()),
        }
    }
}

fn exists_on_path(exec: &str) -> bool {
    let Ok(path_var) = env::var("PATH") else {
        return false;
    };
    let path_var = env::split_paths(&path_var);
    for dir in path_var {
        let test_path = dir.join(exec);
        if test_path.exists() {
            return true;
        }
    }
    false
}

fn main() -> std::io::Result<()> {
    let cli = Cli::parse();
    let mut entries = read_entries()?;
    entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    let mut hidden_entries = Vec::new();
    let mut entries_string = String::new();
    for entry in &entries {
        if entry.hide {
            hidden_entries.push(entry);
            continue;
        } else if hidden_entries.iter().any(|e| e.name == entry.name) {
            entries_string.push_str(entry.field(&cli.entry_type));
            entries_string.push('\n');
        }
    }

    if cli.dmenu.is_none() {
        print!("{}", entries_string);
        return Ok(());
    }

    let dmenu = cli.dmenu.unwrap();
    let Some(mut dmenu_split) = shlex::split(&dmenu) else {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Invalid dmenu command.",
        ));
    };
    let program = dmenu_split.remove(0);
    let mut menu_handle = Command::new(program)
        .args(dmenu_split)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    let _ = menu_handle
        .stdin
        .as_mut()
        .unwrap()
        .write(entries_string.as_bytes());
    let output = String::from_utf8(menu_handle.wait_with_output()?.stdout)
        .expect("Output should be valid UTF8");

    let Some(selected_entry) = entries
        .iter()
        .find(|e| e.field(&cli.entry_type) == output.trim())
    else {
        let Some(mut split) = shlex::split(output.trim()) else {
            return Err(io::Error::new(io::ErrorKind::Other, "Invalid command."));
        };
        let program = split.remove(0);
        let output = Command::new(program).args(split).output()?;
        println!("{}", String::from_utf8_lossy(&output.stdout));
        eprintln!(
            "Command exited with status {}",
            output.status.code().unwrap_or(-1)
        );
        return Ok(());
    };

    let mut command_string = selected_entry.exec.to_owned();
    if cli.terminal.is_some() && selected_entry.terminal {
        let terminal = cli.terminal.unwrap();
        if !terminal.contains("{}") {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Invalid terminal command",
            ));
        }
        command_string = terminal.replace("{}", command_string.as_str());
    }

    let Some(mut exec_split) = shlex::split(command_string.as_str()) else {
        return Err(io::Error::new(io::ErrorKind::Other, "Invalid exec key."));
    };
    let program = exec_split.remove(0);
    let mut command = Command::new(program);
    command.args(exec_split);
    if let Some(path) = &selected_entry.path {
        command.current_dir(path);
    }

    if let Err(e) = command.spawn() {
        eprintln!("Application exited with error: {}", e);
    }
    Ok(())
}

fn read_entries() -> io::Result<Vec<DesktopEntry>> {
    let data_home = match env::var_os("XDG_DATA_HOME") {
        Some(data_home) => PathBuf::from(data_home).join("applications"),
        None => match env::var_os("HOME") {
            Some(home) => PathBuf::from(home).join(".local/share/applications"),
            None => return Err(io::Error::new(io::ErrorKind::Other, "HomeNotFound")),
        },
    };
    let data_dirs = match env::var_os("XDG_DATA_DIRS") {
        Some(dirs) => env::split_paths(&dirs).map(PathBuf::from).collect(),
        None => vec![
            PathBuf::from("/usr/local/share"),
            PathBuf::from("/usr/share"),
        ],
    };

    let mut application_dirs = Vec::new();
    application_dirs.push(data_home);
    for data_dir in data_dirs {
        application_dirs.push(data_dir.join("applications"));
    }

    let mut entries = Vec::new();
    for application_dir in application_dirs {
        let mut new_entries = get_entries(application_dir);
        entries.append(&mut new_entries);
    }

    Ok(entries)
}

fn get_entries<P: AsRef<Path>>(path: P) -> Vec<DesktopEntry> {
    let mut entries: Vec<DesktopEntry> = Vec::new();
    let Ok(applications) = fs::read_dir(path) else {
        return entries;
    };
    for file in applications {
        let Ok(file) = file else { continue };

        let path = file.path();
        let Some(stem) = path.file_stem() else {
            continue;
        };
        let Some(stem) = stem.to_str() else {
            continue;
        };
        let extension = path.extension();
        if extension.is_none() || extension.unwrap() != "desktop" {
            continue;
        }

        let Ok(ini) = Ini::load_from_file_opt(
            &path,
            ini::ParseOption {
                enabled_quote: false,
                enabled_escape: false,
            },
        ) else {
            continue;
        };
        let Some(entry) = DesktopEntry::from_ini(stem, ini) else {
            continue;
        };
        entries.push(entry);
    }
    entries
}
