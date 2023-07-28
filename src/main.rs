use std::{env, fs, path};

#[derive(Debug)]
struct DesktopEntry {}

fn main() {
    // clap
    // collect all the desktop entries and create a vector of structs with them
    let _entries = read_entries();
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

fn get_entries<P: AsRef<path::Path>>(path: P) -> Vec<DesktopEntry> {
    let entries: Vec<DesktopEntry> = Vec::new();
    let Ok(applications) = fs::read_dir(path) else { return entries };
    for entry in applications {
        let Ok(entry) = entry else { continue };
        println!("{:?}", entry);
        let entry = parse_entry(entry.path());
        println!("{:?}", entry);
    }
    entries
}

fn parse_entry(entry: path::PathBuf) -> Option<DesktopEntry> {
    let Ok(contents) = fs::read_to_string(entry) else { return None };
    let mut split = contents.split("\n");
    split.next(); // don't use the first line [Desktop Entry]
    for line in split {
        let mut line = line.split("=");
        let key = line.nth(0);
        let value = line.nth(1);
        
    }
    // DesktopEntry.new(name, exec);
    todo!();
}
