use std::{env, fs};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::io::{self, Write};

use clap::Parser;
use ini::Ini;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Determines the command used to invoke dmenu Executed with your shell ($SHELL) or /bin/sh
    #[arg(long)]
    dmenu: Option<String>,

    /// Must point to a read-writeable file (will create if not exists). In this mode entries are sorted by usage frequency.
    #[arg(long)]
    usage_log: Option<PathBuf>,
}


#[derive(Debug)]
struct DesktopEntry {
    name: String,
    exec: String,
    hide: bool,
    terminal: bool,
    path: Option<String>,
}

impl DesktopEntry {
    fn from_ini(ini: Ini) -> Option<DesktopEntry> {
        let Some(section) = ini.section(Some("Desktop Entry")) else { return None };
        if section.get("Type") != Some("Application") { return None; }
        
        let Some(name) = section.get("Name") else { return None };
        let Some(exec) = section.get("Exec") else { return None };

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

        let hide = !exec_exists || section.get("NoDisplay") == Some("true") || section.get("Hidden") == Some("true");
        
        Some(DesktopEntry {
            name: name.to_owned(),
            exec: exec.to_owned(),
            hide,
            terminal,
            path
        } )
    }
}

fn exists_on_path(exec: &str) -> bool {
    let Ok(path_var) = env::var("PATH") else { return false };
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
    let entries = read_entries()?;

    // 5: sort using usage_log
    let mut entries_string = String::new();
    for entry in &entries {
        // TODO: add support for hide value
        entries_string.push_str(entry.name.as_str());
        entries_string.push_str("\n");
    }

    if let Some(dmenu) = cli.dmenu {
        let Some(mut dmenu_split) = shlex::split(&dmenu) else { return Err(io::Error::new(io::ErrorKind::Other, "Invalid dmenu command.")) };
        let program = dmenu_split.remove(0);
        let mut menu_handle = Command::new(program).args(dmenu_split).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn()?;
        let _ = menu_handle.stdin.as_mut().unwrap().write(entries_string.as_bytes());
        let output = String::from_utf8(menu_handle.wait_with_output()?.stdout).expect("Output should be valid UTF8");

        let Some(selected_entry) = entries.iter().find(|e| e.name == output.trim()) else {
            // run command
            return Ok(());
        };
        
        eprintln!("{:?}", selected_entry);

        // TODO: add support for Terminal key
        let Some(mut exec_split) = shlex::split(selected_entry.exec.as_str()) else { return Err(io::Error::new(io::ErrorKind::Other, "Invalid exec key.")) };
        let program = exec_split.remove(0);
        let mut command = Command::new(program);
        command.args(exec_split);
        if let Some(path) = &selected_entry.path {
            command.current_dir(path);
        }

        // TODO: handle error
        let exec_handle = command.spawn();


        // 4: update usage_log
    } else {
        println!("{}", entries_string);
    }
    Ok(())
}

fn read_entries() -> io::Result<Vec<DesktopEntry>> {
    let data_home = match env::var_os("XDG_DATA_HOME") {
        Some(data_home) => PathBuf::from(data_home).join("applications"),
        None => match env::var_os("HOME") {
            Some(home) => PathBuf::from(home).join(".local/share/applications"),
            None => return Err(io::Error::new(io::ErrorKind::Other, "HomeNotFound")), // maybe later use dirs crate to get home
        }
    };
    let data_dirs = match env::var_os("XDG_DATA_DIRS") {
        Some(dirs) => env::split_paths(&dirs).map(PathBuf::from).collect(),
        None => vec![PathBuf::from("/usr/local/share"), PathBuf::from("/usr/share")],
    };

    let mut application_dirs = Vec::new();
    application_dirs.push(data_home);
    for data_dir in data_dirs {
        application_dirs.push(data_dir.join("applications"));
    }
    eprintln!("{:?}", application_dirs);

    let mut entries = Vec::new();
    for application_dir in application_dirs {
        let mut new_entries = get_entries(application_dir);
        entries.append(&mut new_entries);
    }

    Ok(entries)
}

fn get_entries<P: AsRef<Path>>(path: P) -> Vec<DesktopEntry> {
    let mut entries: Vec<DesktopEntry> = Vec::new();
    let Ok(applications) = fs::read_dir(path) else { return entries };
    for file in applications {
        let Ok(file) = file else { continue };

        let path = file.path();
        let extension = path.extension();
        if extension == None || extension.unwrap() != "desktop" {
            continue;
        }

        let Ok(ini) = Ini::load_from_file_opt(path, ini::ParseOption { enabled_quote: false, enabled_escape: false } ) else { continue };
        let Some(entry) = DesktopEntry::from_ini(ini) else { continue };
        eprintln!("{:?}", entry);
        entries.push(entry);
    }
    entries
}

