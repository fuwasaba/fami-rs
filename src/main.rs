#![windows_subsystem = "windows"]
use std::{
    fs,
    io::{Read, Seek},
};

use gtk::prelude::*;
use home::home_dir;
use rand::distributions::{Alphanumeric, DistString};
use serde_json::json;
const VERSION: &str = "1.18.2-forge-40.2.14";
const VERSION_COMPACT: &str = "40.2.14";
const DOWNLOAD_DIR: &str = "https://media-uploader.work/?mode=dl&id=12196&original=1&key=3de8209d-270c-4a8b-8490-a9c93ad0fc79";
const URL_FORGE: &str = "https://files.minecraftforge.net/net/minecraftforge/forge/index_1.18.2.html#:~:text=Mdk-,40.2.14,-2023%2D11%2D08";
#[tokio::main]
async fn main() {
    let application = gtk::Application::new(Some("com.keikun1215"), Default::default());
    application.connect_activate(build_ui);
    application.run();
}
fn build_ui(application: &gtk::Application) {
    let window = gtk::ApplicationWindow::new(application);
    window.set_icon_name(Option::None);
    window.set_resizable(false);
    window.set_title(Some("Installer"));
    window.set_default_size(500, 450);
    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let button = gtk::Button::builder()
        .label("Start")
        .margin_start(50)
        .margin_end(50)
        .margin_top(50)
        .build();
    let label = gtk::Label::builder()
        .label("ようこそ？")
        .margin_top(50)
        .build();
    let label_selected = gtk::Label::builder().label("").margin_top(50).build();
    let label_progress = gtk::Label::builder().label("").margin_top(50).build();
    let chooser = gtk::FileChooserNative::builder()
        .accept_label("Open Folder")
        .title("Open Folder")
        .build();
    chooser.set_action(gtk::FileChooserAction::SelectFolder);
    chooser.connect_response(gtk::glib::clone!(@strong label_selected, @strong button, @strong label_progress, @strong label =>
            move |dir, res_type| {
            if res_type != gtk::ResponseType::Accept {
                return;
            };
            if let Some(directory) = dir.file() {
                label_selected.set_text(&format!("Selected: {}", directory.parse_name().as_str()));
                button.set_label("Next");
                label.set_text("起動構成を注入します。");
                label_progress.set_text("");
            }
        }
    ));
    chooser.hide();
    let (sender, receiver) = async_channel::unbounded();
    button.connect_clicked(
        gtk::glib::clone!(@strong label, @strong label_progress, @strong label_selected, @strong chooser, @strong application =>
            move |button_clicked| {
                if let Some(label_now) = button_clicked.label() {
                    if label_now.to_string() == String::from("Next (ファイルを選択)") {
                        return chooser.show();
                    } else if label_now.to_string() == String::from("Next") || label.text().to_string() == String::from("起動構成を注入します。") {
                        let result = make_profile(&label_selected.text().as_str().replacen("Selected: ", "", 1));
                        if result == 1 {
                            label.set_text("構成modをダウンロードしますよ");
                            label_progress.set_text("");
                            button_clicked.set_label("OK (ファイルがダウンロードされます)");
                        }
                        return;
                    } else if label_now.to_string() == String::from("OK (ファイルがダウンロードされます)") {
                        let sender = sender.clone();
                        let select_label_clone = label_selected.clone().text().as_str().replacen("Selected: ", "", 1);
                        label.set_text("ファイルをダウンロードしています...");
                        button_clicked.set_label("Finish");
                        button_clicked.set_sensitive(false);
                        gtk::gio::spawn_blocking(move || {
                            sender.send_blocking(select_label_clone).expect("fuck");
                        });
                        return;
                    } else if label_now.to_string() == String::from("Finish") || label_now.to_string() == String::from("Close") {
                        application.quit();
                        return;
                    }
                }
                let result = &check_forge(&label_progress);
                label.set_text(result);
                if result.eq(&format!("{}の存在を確認しました。", VERSION)) {
                    label_progress.set_text("マイクラの構成にするディレクトリを選べ");
                    button_clicked.set_label("Next (ファイルを選択)");
                } else if result.eq(&format!("バージョン{}が見つかりませんでした。インストーラをダウンロードし実行してください。", VERSION)) {
                    label_progress.set_text(&format!("開いたリンクからハイライトされているバージョン {} のインストーラを実行し、もう一度私を実行してください。", VERSION_COMPACT));
                    open::that(URL_FORGE).expect("fuck");
                    button_clicked.set_label("Close");
                } else if result.eq("Javaが導入されていないようです。") {
                    label_progress.set_text("開いたリンクからJavaのインストーラを実行し、もう一度私を実行してください。");
                    open::that("https://www.java.com/ja/download/windows_offline.jsp").expect("fuck");
                    button_clicked.set_label("Close");
                }
            }
        ),
    );
    glib::spawn_future_local(
        glib::clone!(@weak label, @strong label_progress, @strong button => async move {
            while let Ok(dir) = receiver.recv().await {
                download_files(&dir, &label_progress).await;
                label.set_text("多分できたよ");
                button.set_sensitive(true);
            }
        }),
    );
    vbox.append(&label);
    vbox.append(&label_progress);
    vbox.append(&button);
    vbox.append(&label_selected);
    window.set_child(Some(&vbox));
    window.show();
}
async fn download_files(game_dir: &str, progress: &gtk::Label) {
    progress.set_text("Initial variables");
    let random_tmpname = Alphanumeric.sample_string(&mut rand::thread_rng(), 10);
    let directory = std::path::Path::new(game_dir);
    progress.set_text("Download configuration");
    let directory_zip: reqwest::Response = reqwest::get(DOWNLOAD_DIR).await.expect("fuck");
    let bytes = directory_zip.bytes().await.expect("fuck");
    progress.set_text("Create temp zip file");
    let mut file = fs::File::options()
        .write(true)
        .read(true)
        .create(true)
        .open(&format!("_{}.zip", random_tmpname))
        .expect("fuck");
    progress.set_text("Writing byte to temp zip file");
    std::io::copy(&mut bytes.as_ref(), &mut file).expect("fuck");
    let mut zip = zip::ZipArchive::new(file).expect("fuck");
    progress.set_text("Copy jar file from temp zip file: ");
    for i in 0..zip.len() {
        let mut file = zip.by_index(i).expect("fuck");
        let dirname_and_file = file.name().replacen("ふわ鯖1期構成/", "", 1);
        progress.set_text(&format!(
            "Copy jar file from temp zip file: {}",
            dirname_and_file
        ));
        if file.is_dir() {
            continue;
        };
        let mut without_file_vec: Vec<&str> = dirname_and_file.split("/").collect();
        without_file_vec.pop().expect("fuck");
        let without_file = without_file_vec.join("/").to_string();
        fs::create_dir_all(directory.join(without_file)).expect("fuck");
        let mut new_downloaded = fs::File::options()
            .write(true)
            .read(false)
            .create(true)
            .open(directory.join(dirname_and_file))
            .expect("fuck");
        std::io::copy(file.by_ref(), &mut new_downloaded).expect("fuck");
    }
    progress.set_text(&format!(
        "Remove temporary zip file: _{}.zip",
        random_tmpname
    ));
    std::fs::remove_file(&format!("_{}.zip", random_tmpname)).expect("fuck");
}
fn make_profile(game_dir: &str) -> u8 {
    if let Some(path) = home_dir() {
        let profiles_path = path
            .join("AppData")
            .join("Roaming")
            .join(".minecraft")
            .join("launcher_profile.json");
        let profiles = std::io::BufReader::new(std::fs::File::open(profiles_path).unwrap());
        let json = serde_json::from_reader::<std::io::BufReader<std::fs::File>, serde_json::Value>(
            profiles,
        );
        let mut json: serde_json::Value = match json {
            Ok(o) => o,
            Err(_) => {
                println!("Failed to parse json.");
                return 0;
            }
        };
        json["profiles"]["Fuwasaba-ss1"] = json!({
            "created": chrono::Utc::now().to_string(),
            "icon" : "TNT",
            "lastUsed": "1970-01-01T00:00:00.000Z",
            "lastVersionId": VERSION,
            "name": "Fuwasaba-ss1",
            "type": "custom",
            "gameDir": game_dir
        });
        let mut writer = std::io::BufWriter::new(
            std::fs::File::options()
                .read(false)
                .write(true)
                .append(false)
                .open(
                    path.join("AppData")
                        .join("Roaming")
                        .join(".minecraft")
                        .join("launcher_profiles.json"),
                )
                .unwrap(),
        );
        writer.seek(std::io::SeekFrom::Start(0)).expect("fuck");
        serde_json::to_writer(writer, &json).expect("fuck");
    }
    1
}
fn check_forge(label: &gtk::Label) -> String {
    label.set_text("Checking for home_dir...");
    match std::process::Command::new("java").output() {
        Ok(o) => o,
        Err(err) => {
            println!("{}", err);
            return String::from("Javaが導入されていないようです。");
        }
    };
    if let Some(path) = home_dir() {
        label.set_text("Checking for Roaming directory...");
        let roaming = path.join("AppData").join("Roaming");
        if roaming.is_dir() {
            let dot_minecraft = roaming.join(".minecraft");
            label.set_text("Checking for .minecraft directory...");
            if dot_minecraft.is_dir() {
                let need_version = dot_minecraft.join("versions").join(VERSION);
                label.set_text("Checking for version forge...");
                if need_version.is_dir() {
                    String::from(format!("{}の存在を確認しました。", VERSION))
                } else {
                    String::from(format!("バージョン{}が見つかりませんでした。インストーラをダウンロードし実行してください。", VERSION))
                }
            } else {
                String::from("WTF .minecraft not found.")
            }
        } else {
            String::from("WTF AppData\\Roaming not found")
        }
    } else {
        String::from("WTF home_dir() exception occured")
    }
}
