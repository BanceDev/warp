use gdk4_x11::X11Display;
use gdk4_x11::X11Surface;
use gdk4_x11::x11::xlib::{PropModeReplace, XA_ATOM, XChangeProperty, XInternAtom};
use gtk::CssProvider;
use gtk::gdk::Display;
use gtk::glib;
use gtk::prelude::*;
use std::ffi::CString;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;
use x11::xlib;
use xdg::BaseDirectories;

static APP_ID: &str = "dev.eidolon.edock";

#[derive(Debug, Clone)]
struct App {
    icon: Option<String>,
    command: Option<String>,
}

fn set_utf8_props(window: &gtk::Window, prop_name: &str, prop_value: &str) {
    let surface = window.surface().unwrap();
    unsafe {
        let xsurf = surface.unsafe_cast::<X11Surface>();
        xsurf.set_utf8_property(prop_name, Some(prop_value));
    }
}

fn set_window_props(window: &gtk::Window, prop_name: &str, prop_values: &Vec<&str>) {
    let display = WidgetExt::display(window);
    let surface = window.surface().unwrap();
    let prop_name_cstr = CString::new(prop_name).unwrap();
    let prop_values_cstr: Vec<CString> = prop_values
        .iter()
        .map(|val| CString::new(*val).unwrap())
        .collect();
    unsafe {
        let xid: xlib::Window = surface.unsafe_cast::<X11Surface>().xid();
        let xdisplay: *mut xlib::Display = display.unsafe_cast::<X11Display>().xdisplay();
        let prop_name_atom = XInternAtom(xdisplay, prop_name_cstr.as_ptr(), xlib::False);
        let mut prop_values_atom: Vec<u64> = prop_values_cstr
            .into_iter()
            .map(|cstr| XInternAtom(xdisplay, cstr.as_ptr(), xlib::False))
            .collect();
        let num_values = prop_values_atom.len();
        let prop_values_c = prop_values_atom.as_mut_ptr();
        XChangeProperty(
            xdisplay,
            xid,
            prop_name_atom,
            XA_ATOM,
            32,
            PropModeReplace,
            prop_values_c as *const u8,
            num_values as i32,
        );
    }
}

fn find_icon(icon_name: &str, icon_theme: Option<&str>, icon_size: Option<i64>) -> Option<String> {
    // Use a default theme if icon_theme is None
    let theme = icon_theme.unwrap_or("");
    let size = match icon_size {
        Some(i) => format!("{}x{}", i, i),
        None => "".to_string(),
    };

    // Build icon directories
    let icon_dirs = [
        &Path::new("/usr/share/icons").join(theme).join(size),
        &Path::new("/usr/share/icons").join(theme),
        Path::new("/usr/share/icons"),
        &Path::new(&dirs::home_dir().unwrap()).join(".icons"),
    ];
    let icon_exts = ["svg", "xpm", "png"];

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

fn app_clicked(command: String) {
    std::process::Command::new(&command)
        .spawn()
        .expect("failed to execute app");
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
    let icon_theme = config["icon_theme"].as_str();
    let icon_size = config["icon_size"].as_i64();

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
                    app.icon = find_icon(line[5..].into(), icon_theme, icon_size);
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

    dock_window.connect_realize(move |dock_window| {
        set_window_props(
            dock_window.upcast_ref(),
            "_NET_WM_WINDOW_TYPE",
            &vec!["_NET_WM_WINDOW_TYPE_DOCK"],
        );
        set_utf8_props(dock_window.upcast_ref(), "_OB_APP_TYPE", "dock");
        dock_window.present();
    });

    let button_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    button_box.set_css_classes(&["button"]);

    for app in apps {
        let button = gtk::Button::new();
        button.set_css_classes(&["app"]);
        if let Some(icon) = app.icon {
            let image = gtk::Image::from_file(&icon);
            button.set_child(Some(&image));
        }
        if let Some(command) = app.command {
            button.connect_clicked(move |button| {
                app_clicked(command.clone());
                button.set_css_classes(&["app_open"]);
            });
        }
        button.set_size_request(button_height, button_height);
        button_box.append(&button);
    }

    dock_window.set_child(Some(&button_box));
    WidgetExt::realize(&dock_window);
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
