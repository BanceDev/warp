use gtk::CssProvider;
use gtk::gdk::Display;
use gtk::glib;
use gtk::prelude::*;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;
use xdg::BaseDirectories;

static APP_ID: &str = "dev.eidolon.edock";

#[derive(Debug, Clone)]
struct App {
    icon: Option<String>,
    command: Option<String>,
}

fn find_icon(icon_name: &str) -> Option<String> {
    let icon_dirs = [
        Path::new("/usr/share/icons"),
        &Path::new(&dirs::home_dir().unwrap()).join(".icons"),
    ];
    let icon_exts = ["png", "svg", "xpm"];

    for dir in &icon_dirs {
        for ext in &icon_exts {
            let icon_file_name = format!("{}.{}", icon_name, ext);
            for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
                if entry.file_name().to_string_lossy() == icon_file_name {
                    return Some(entry.path().to_path_buf().to_string_lossy().to_string());
                }
            }
        }
    }
    None
}

fn build_ui(app: &gtk::Application) {
    // read config json
    let base_dirs = BaseDirectories::with_prefix("edock").unwrap();
    let config_path = base_dirs.find_config_file("config").unwrap();
    let config_content = fs::read_to_string(&config_path).unwrap();
    let config: serde_json::Value = serde_json::from_str(&config_content).unwrap();
    let app_names = config["apps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap())
        .collect::<Vec<&str>>();
    let button_height = config["button_height"].as_i64().unwrap() as i32;

    let base_dirs = BaseDirectories::with_prefix("applications").unwrap();

    // get a collection of app info from their .desktop files
    let apps = {
        let mut apps = Vec::new();
        for app_name in &app_names {
            let mut app = App {
                icon: None,
                command: None,
            };

            let entry_path = base_dirs
                .find_data_file(format!("{}.desktop", app_name))
                .unwrap();
            let content = fs::read_to_string(&entry_path).unwrap();
            let lines: Vec<&str> = content.lines().collect();

            for line in lines {
                if line.starts_with("Icon=") {
                    app.icon = find_icon(line[5..].into());
                }
                if line.starts_with("Exec=") {
                    app.command = Some(line[5..].split(' ').next().unwrap().into());
                }
            }
            apps.push(app);
        }
        apps
    };

    let dock_window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("edock")
        .decorated(false)
        .resizable(false)
        .css_classes(["edock"])
        .build();

    let button_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    button_box.set_css_classes(&["button"]);

    for app in apps {
        let button = gtk::Button::new();
        button.set_css_classes(&["app"]);
        if let Some(icon) = app.icon {
            let image = gtk::Image::from_file(&icon);
            button.set_child(Some(&image));
        }
        if let Some(command) = app.command {
            button.connect_clicked(move |_| {
                std::process::Command::new(&command)
                    .spawn()
                    .expect("failed to execute app");
            });
        }
        button.set_size_request(button_height, button_height);
        button_box.append(&button);
    }

    dock_window.set_child(Some(&button_box));
    dock_window.present();
}

fn main() -> glib::ExitCode {
    gtk::init().expect("Failed to initialize GTK.");
    let app = gtk::Application::new(Some(APP_ID), Default::default());

    // load styles
    let provider = CssProvider::new();
    let base_dirs = BaseDirectories::with_prefix("edock").unwrap();
    let style_path = base_dirs.find_config_file("style.css").unwrap();
    provider.load_from_path(style_path);
    let display = Display::default().expect("Could not connect to a display.");

    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    app.connect_activate(build_ui);
    app.run()
}
