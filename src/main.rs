use std::{env, fs};

struct DesktopEntry {}

fn main() {
    // clap
    // collect all the desktop entries and create a vector of structs with them
    let _entries = read_entries();
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

    let entries = Vec::new();
    for application_folder in application_folders {
        entries.append(get_entries(application_folder));
    }

    Some(entries)
}

fn get_entries<P: AsRef<Path>>(path: P) -> Vec<DesktopEntry> {
    let entries = Vec::new();
    let Ok(applications) = fs::read_dir(application_folder) else { continue };
    for desktop_entry in applications {
        let Ok(desktop_entry) = desktop_entry else { continue };
        println!("{:?}", desktop_entry);
        if desktop_entry.file_type().is_dir() {
            entries.append(get_entries(desktop_entry.path()));
        } else if desktop_entry.file_type().is_file() {
            // create desktop entry
            let entry;
            entries.push(entry);
        } else {
            continue;
        }
    }
}
