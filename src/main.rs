use std::{env, fs};
use std::path::Path;
use std::collections::HashMap;
use std::process::Command;

#[derive(Debug)]
struct DesktopEntry {
    name: String,
    hide: bool,
    command: Command,
}

impl DesktopEntry {
    fn new(name: String, hash_map: HashMap<String, String>) -> Option<DesktopEntry> {
        let nothing = String::from("");

        let Some(exec) = hash_map.get(&String::from("Exec")) else { return None };
        let no_display = hash_map.get(&String::from("NoDisplay")).unwrap_or(&nothing);
        let hidden = hash_map.get(&String::from("Hidden")).unwrap_or(&nothing);
        // let try_exec = hash_map.get(&String::from("TryExec"));
        let path = hash_map.get(&String::from("Path"));
        let terminal = hash_map.get(&String::from("Terminal"));

        // test if tryexec is here, else don't make visible / skip entry
        // if let Some(try_exec) = try_exec {
        //     let path = Path::new(try_exec);
        //     if !path.exists() {
                
        //     }
        // }

        let hide = no_display == "true" || hidden == "true";
        let mut exec = exec.split(" ");
        let exec_path = exec.nth(0).expect("exec should contain text").to_string();
        let mut command;

        if terminal.is_none() {
            command = Command::new(exec_path);
        } else {
            command = Command::new("foot");
            command.arg(exec_path);
        }

        if let Some(path) = path {
            if !path.is_empty() {
                command.current_dir(path);
            }
        }

        Some(DesktopEntry { name, hide, command })
    }
}

fn main() {
    // clap
    let entries = read_entries();
    // clear up the entries
    // run dmenu and wait for the output
    // when dmenu returns, run the command in the struct
}

fn read_entries() -> Option<Vec<DesktopEntry>> {
    let home_applications = match env::var("HOME") {
        Ok(mut home) => {
            home.push_str("/.local/share/applications");
            home
        },
        Err(_) => return None,
    };

    let Ok(xdg_data_dirs) = env::var("XDG_DATA_DIRS") else { return None };
    let data_dirs = xdg_data_dirs.split(":").collect::<Vec<&str>>();
    let mut application_folders = Vec::new();
    for dir in data_dirs {
        let mut dir = dir.to_owned();
        dir.push_str("/applications");
        application_folders.push(dir);
    }
    application_folders.insert(0, home_applications);
    println!("{:?}", application_folders);

    let mut entries = Vec::new();
    for application_folder in application_folders {
        let mut new_entries = get_entries(application_folder);
        entries.append(&mut new_entries);
    }

    Some(entries)
}

fn get_entries<P: AsRef<Path>>(path: P) -> Vec<DesktopEntry> {
    let entries: Vec<DesktopEntry> = Vec::new();
    let Ok(applications) = fs::read_dir(path) else { return entries };
    for entry in applications {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        let extension = path.extension();
        if extension == None && extension.unwrap() != "desktop" {
            continue;
        }

        println!("{:?}", entry);
        let entry = parse_entry(path);
        println!("{:?}", entry);
    }
    entries
}

fn parse_entry(entry: std::path::PathBuf) -> Option<DesktopEntry> {
    let Ok(contents) = fs::read_to_string(entry) else { return None };
    let mut split = contents.split("\n");
    split.next(); // don't use the first line [Desktop Entry]

    let mut hash_map: HashMap<String, String> = HashMap::new();
    for line in split {
        if line.starts_with("[") && line.ends_with("]") { continue }
        let mut line = line.split("=");
        let key = line.nth(0);
        let value = line.nth(0);
        if let None = value { continue; } // hack
        hash_map.insert(key.unwrap().to_string(), value.unwrap().to_string());
    }
    // check if type is application
    let Some(kind) = hash_map.remove(&String::from("Type")) else { return None };
    if kind != "Application" { return None }
    let Some(name) = hash_map.remove(&String::from("Name")) else { return None };
    
    DesktopEntry::new(name, hash_map)
}
