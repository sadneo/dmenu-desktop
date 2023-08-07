use std::{env, fs};
use std::path::{Path, PathBuf};

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
    try_exec: Option<String>,
    path: Option<String>,
}

impl DesktopEntry {
    fn from_ini(ini: Ini) -> Option<DesktopEntry> {
        let Some(section) = ini.section(Some("Desktop Entry")) else { return None };
        if section.get("Type") != Some("Application") { return None; }
        
        let Some(name) = section.get("Name") else { return None };
        let Some(exec) = section.get("Exec") else { return None };
        let hide = section.get("NoDisplay") == Some("true") || section.get("Hidden") == Some("true");

        let try_exec = section.get("TryExec").map(str::to_string);
        let path = section.get("Path").map(str::to_string);
        // TODO: Later add support for Terminal key
        
        Some(DesktopEntry {
            name: name.to_owned(),
            exec: exec.to_owned(),
            hide,
            try_exec,
            path
        } )
    }
}

#[derive(Debug)]
enum ErrorKind {
    HomeNotFound,
}

fn main() {
    let cli = Cli::parse();
    let entries = match read_entries() {
        Ok(t) => t,
        Err(error_kind) => match error_kind {
            ErrorKind::HomeNotFound => {
                eprintln!("$HOME not found");
                return;
            }
        }
    };

    // 5: sort using usage_log

    if let Some(dmenu) = cli.dmenu {
        // 2: run dmenu and wait for the output
        // 3: when dmenu returns, run the command in the struct

        /*
        test if tryexec is here, else don't make visible / skip entry
        if let Some(exec_path) = try_exec {
            look through path
        }

        let Some(exec_split) = shlex::split(exec) else { return None };
        let program = exec_split.remove(0);
        let command = Command::new(program).args(exec_split);

        if let Some(path) = path {
            command.current_dir(path);
        }
        */


        // 4: update usage_log
    } else {
        // 1: print to stdout
    }
}

fn read_entries() -> Result<Vec<DesktopEntry>, ErrorKind> {
    let data_home = match env::var_os("HOME") {
        Some(home) => PathBuf::from(home).join(".local/share/applications"),
        None => return Err(ErrorKind::HomeNotFound), // maybe later use dirs crate to get home
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

