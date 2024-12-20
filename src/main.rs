use std::collections::HashMap;
use std::io::{self, Write};
use std::path::PathBuf;
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

    /// Determines the command used to invoke dmenu or an equivalent.
    #[arg(long)]
    dmenu: Option<String>,

    /// Terminal emulator used to launch applications, does nothing if dmenu is not provided, put {} where the dmenu command should go
    #[arg(long)]
    terminal: Option<String>,
}

#[derive(Debug, Hash, PartialEq, Eq)]
struct DesktopEntry {
    name: String,
    filename: String,
    exec: String,
    hide: bool,
    terminal: bool,
    path: Option<PathBuf>,
}

impl DesktopEntry {
    fn from_ini(filename: &str, ini: Ini) -> Option<DesktopEntry> {
        let section = ini.section(Some("Desktop Entry"))?;
        if section.get("Type") != Some("Application") {
            return None;
        }

        let name = section.get("Name")?;
        let exec = section.get("Exec")?;

        let try_exec = section.get("TryExec");
        let path = section.get("Path").map(PathBuf::from);
        let terminal = section.get("Terminal") == Some("true");

        let exec_exists = match try_exec {
            Some(ref exec_path) => match PathBuf::from(exec_path).exists() {
                true => true,
                false => {
                    let path_var = env::var("PATH").expect("$PATH should be defined");
                    env::split_paths(&path_var)
                        .map(|p| p.join(exec))
                        .any(|p| p.exists())
                }
            },
            None => true,
        };

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

fn main() -> std::io::Result<()> {
    let cli = Cli::parse();
    let mut entries: Vec<DesktopEntry> = read_entries().into_values().collect();
    entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    let entries_string = entries
        .iter()
        .filter(|e| !e.hide)
        .map(|e| e.field(&cli.entry_type))
        .fold(String::new(), |mut acc, field| {
            acc.push_str(field);
            acc.push('\n');
            acc
        });

    if cli.dmenu.is_none() {
        print!("{}", entries_string);
        Ok(())
    } else {
        run_command(cli, entries, entries_string)
    }
}

fn run_command(cli: Cli, entries: Vec<DesktopEntry>, entries_string: String) -> io::Result<()> {
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

/// Returns a hash set of desktop entries in arbitrary order
fn read_entries() -> HashMap<String, DesktopEntry> {
    let mut app_dirs = Vec::new();
    match env::var_os("XDG_DATA_HOME") {
        Some(data_home) => app_dirs.push(PathBuf::from(data_home).join("applications")),
        None => {
            if let Some(home) = env::var_os("HOME") {
                app_dirs.push(PathBuf::from(home).join(".local/share/applications"));
            }
        }
    };
    match env::var_os("XDG_DATA_DIRS") {
        Some(dirs) => env::split_paths(&dirs)
            .map(PathBuf::from)
            .map(|p| p.join("applications"))
            .collect(),
        None => vec![
            PathBuf::from("/usr/local/share/applications"),
            PathBuf::from("/usr/share/applications"),
        ],
    }
    .into_iter()
    .for_each(|p| app_dirs.push(p));

    let mut entries = HashMap::new();
    for app_dir in app_dirs {
        let Ok(apps) = fs::read_dir(app_dir) else {
            continue;
        };
        for file in apps {
            let Ok(file) = file else {
                break;
            };

            let path = file.path();
            let stem = path.file_stem().unwrap().to_str().unwrap();
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

            let stem = stem.to_owned();
            entries.entry(stem).or_insert(entry);
        }
    }

    entries
}
