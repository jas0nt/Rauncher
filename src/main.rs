use fzf_wrapped::run_with_output;
use fzf_wrapped::Fzf;
use fzf_wrapped::{Color, Layout};
use std::collections::HashMap;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::Duration;

const XDG_DATA_HOME: &str = "XDG_DATA_HOME";
const XDG_DATA_DIRS: &str = "XDG_DATA_DIRS";
const APP_FOLDER: &str = "applications";

#[derive(Debug)]
struct DesktopEntry {
    name: String,
    exec: String,
}

fn main() {
    let desktop_files = get_all_desktop_files();
    let de_maps = parse_desktop_files(desktop_files);
    let names = de_maps.keys().cloned().collect::<Vec<_>>();

    if let Some(selection) = run_fzf(names) {
        match de_maps.get(&selection) {
            Some(de) => run_detached_command(&de.exec),
            _ => println!("No cmd"),
        }
    } else {
        println!("No Selection");
    }
}

fn run_detached_command(command: &str) {
    println!("Running {}", command);
    let child = Command::new("sh")
        .arg("-c")
        .arg(format!("setsid {} > /dev/null 2>&1 &", command))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn();
    sleep(Duration::from_millis(10));
    std::mem::drop(child);
}

fn remove_placeholders(input: &str) -> String {
    let placeholders: [&str; 11]= [" %f", " %F", " %u", " %U", " %d", " %D", " %n", " %N", " %i", " %c", " %k"];
    let pattern = placeholders.join("|");
    let re = regex::Regex::new(&pattern).unwrap();
    re.replace_all(input, "").to_string()
}

fn run_fzf(names: Vec<String>) -> Option<String> {
    let fzf = Fzf::builder()
        .layout(Layout::Default)
        .color(Color::Dark)
        .build()
        .unwrap();

    let selection = run_with_output(fzf, names);
    // println!("User selection: {:?}", selection);
    if let Some(name) = selection {
        Some(name)
    } else {
        None
    }
}

fn parse_desktop_files(files: Vec<PathBuf>) -> HashMap<String, DesktopEntry> {
    let mut result: HashMap<String, DesktopEntry> = HashMap::new();
    for file in files {
        if let Some(de) = parse_desktop_file(file) {
            result.insert(de.name.clone(), de);
        }
    }
    result
}

fn parse_desktop_file(path: PathBuf) -> Option<DesktopEntry> {
    // println!("Parsing {:?}", path);
    match fs::read_to_string(path) {
        Ok(content) => {
            let mut name = None;
            let mut exec = None;
            for line in content.lines() {
                if line.starts_with("Name=") {
                    name = Some(line.trim_start_matches("Name=").to_string());
                } else if line.starts_with("Exec=") {
                    exec = Some(line.trim_start_matches("Exec=").to_string());
                }
                if name.is_some() && exec.is_some() {
                    break;
                }
            }

            if let (Some(mut name), Some(mut exec)) = (name, exec) {
                exec = remove_placeholders(&exec);
                name = format!("{} ({})", name, exec).to_string();
                return Some(DesktopEntry {
                    name,
                    exec,
                });
            }
        }
        Err(e) => {
            eprintln!("Fail reading file {:?}", e);
        }
    };
    None
}

fn get_all_desktop_files() -> Vec<PathBuf> {
    let xdg_data_home = env::var(XDG_DATA_HOME)
        .unwrap_or_else(|_| format!("{}/.local/share", env::var("HOME").unwrap_or_default()));
    // println!("{XDG_DATA_HOME}: {}", xdg_data_home);

    let xdg_data_dirs =
        env::var(XDG_DATA_DIRS).unwrap_or_else(|_| "/usr/local/share:/usr/share".to_string());
    // println!("{XDG_DATA_DIRS}: {}", xdg_data_dirs);

    let xdg_data_dir_arr: Vec<&str> = xdg_data_dirs.split(":").collect();
    // println!("{XDG_DATA_DIRS} array: {:?}", xdg_data_dir_arr);

    let data_home = PathBuf::from(&xdg_data_home).join(APP_FOLDER);
    let mut desktop_files = match find_desktop_files(&data_home) {
        Ok(files) => files,
        Err(_) => Vec::new(),
    };

    for dir in xdg_data_dir_arr {
        let path = PathBuf::from(&dir).join(APP_FOLDER);
        let files = match find_desktop_files(&path) {
            Ok(vals) => vals,
            Err(_) => Vec::new(),
        };
        desktop_files.extend(files);
    }

    return desktop_files;
}

fn find_desktop_files(path: &Path) -> Result<Vec<PathBuf>, io::Error> {
    // println!("finding in {:?}", path);
    let mut desktop_files: Vec<PathBuf> = Vec::new();

    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension() == Some(OsStr::new("desktop")) {
                desktop_files.push(path);
            } else if path.is_dir() {
                // Recursively search subdirectories
                desktop_files.extend(find_desktop_files(&path)?);
            }
        }
    }

    Ok(desktop_files)
}
