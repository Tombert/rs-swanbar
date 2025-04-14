use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use std::result::Result as StdResult;
use std::time::Duration;

pub type BoxedHandler = fn() -> Pin<
    Box<
        dyn Future<Output = StdResult<HashMap<String, String>, Box<dyn Error + Send + Sync>>>
            + Send,
    >,
>;

pub type MouseBoxedHandler = fn() -> Pin<
    Box<
        dyn Future<Output = StdResult<(), Box<dyn Error + Send + Sync>>>
            + Send,
    >,
>;


pub type RenderFn = fn(&HashMap<String, String>) -> String;

#[derive(Clone, Serialize, Deserialize)]
pub struct Meta {
    pub is_processing: bool,
    pub start_time: Duration,
    pub data: HashMap<String, String>,
}

pub mod volume_click {

    use std::error::Error;
    use std::result::Result as StdResult;
    use tokio::process::Command;
    pub async fn click_handle() -> StdResult<(), Box<dyn Error + Send + Sync>> {
        Command::new("pavucontrol").output().await?;
        Ok(())
    }
}

pub mod wifi_click {

    use std::error::Error;
    use std::result::Result as StdResult;
    use tokio::process::Command;
    pub async fn click_handle() -> StdResult<(), Box<dyn Error + Send + Sync>> {
        Command::new("pkill").arg("iwgtk").output().await?;
        Command::new("iwgtk").output().await?;

        Ok(())
    }
}

//pub struct MouseNoop;

pub mod mouse_noop {
    use std::result::Result as StdResult;
    use std::error::Error;
    pub async fn click_handle() -> StdResult<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }
}

pub mod bg_changer {
    use rand::Rng;
    use rand::SeedableRng;
    use rand::rngs::StdRng;
    use std::collections::HashMap;
    use std::error::Error;
    use std::result::Result as StdResult;
    use tokio::fs;
    use tokio::process::Command;
    use tokio_stream::StreamExt;
    use tokio_stream::wrappers::ReadDirStream;
    pub async fn handle() -> StdResult<HashMap<String, String>, Box<dyn Error + Send + Sync>> {
        let mut entries = ReadDirStream::new(fs::read_dir("/home/tombert/wallpapers/").await?);
        let mut files = Vec::new();

        while let Some(entry) = entries.next().await {
            let entry = entry?;
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();
            if file_name_str.ends_with(".jpg")
                || file_name_str.ends_with(".jpeg")
                || file_name_str.ends_with(".png")
            {
                files.push(file_name);
            }
        }

        let mut rng = StdRng::from_entropy();
        let random_num = rng.gen_range(0..files.len());
        let image = files[random_num].to_string_lossy().to_string();
        let image = format!("/home/tombert/wallpapers/{}", image);
        Command::new("pkill").arg("swaybg").output().await?;
        tokio::spawn(async {
            let _ = Command::new("swaybg")
                .arg("-i")
                .arg(image)
                .arg("-m")
                .arg("stretch")
                .output()
                .await;
        });

        let mut out_hash = HashMap::new();
        out_hash.insert("".to_string(), "".to_string());
        Ok(out_hash)
    }
    pub fn render(_i: &HashMap<String, String>) -> String {
        "".to_string()
    }
}

pub mod noop {

    use std::collections::HashMap;
    use std::error::Error;
    use std::result::Result as StdResult;

    pub async fn handle() -> StdResult<HashMap<String, String>, Box<dyn Error + Send + Sync>> {
        let mut out_hash = HashMap::new();
        out_hash.insert("".to_string(), "".to_string());
        Ok(out_hash)
    }
    pub fn render(_i: &HashMap<String, String>) -> String {
        "".to_string()
    }
}

pub mod current_program {
    use std::collections::HashMap;
    use std::error::Error;
    use std::result::Result as StdResult;
    use swayipc::{Connection, Node};

    fn find_focused(node: &Node) -> Option<&Node> {
        if node.focused {
            return Some(node);
        }
        for child in &node.nodes {
            if let Some(found) = find_focused(child) {
                return Some(found);
            }
        }
        for child in &node.floating_nodes {
            if let Some(found) = find_focused(child) {
                return Some(found);
            }
        }
        None
    }

    pub async fn handle() -> StdResult<HashMap<String, String>, Box<dyn Error + Send + Sync>> {
        let mut connection = Connection::new().expect("Failed to connect to sway");
        let tree = connection.get_tree().expect("Failed to get tree");

        let om = if let Some(focused) = find_focused(&tree) {
            let app_id = focused.app_id.as_deref();
            let class = focused
                .window_properties
                .as_ref()
                .and_then(|props| props.class.as_deref());
            let name = app_id.or(class).unwrap_or("unknown");
            let out_map: HashMap<String, String> = [("out", name)]
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            out_map
        } else {
            let out_map: HashMap<String, String> = [("out", "nothing")]
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            out_map
        };

        Ok(om)
    }
    pub fn render(i: &HashMap<String, String>) -> String {
        format!(
            "{}",
            i.get(&"out".to_string()).unwrap_or(&"nada".to_string())
        )
    }
}

pub mod quote {
    use rand::Rng;
    use rand::SeedableRng;
    use rand::rngs::StdRng;
    use reqwest::Client;
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    use std::error::Error;
    use std::result::Result as StdResult;

    #[derive(Serialize)]
    struct Message {
        role: String,
        content: String,
    }

    #[derive(Serialize)]
    struct ChatRequest {
        model: String,
        messages: Vec<Message>,
    }

    #[derive(Deserialize)]
    struct Choice {
        message: MessageContent,
    }

    #[derive(Deserialize)]
    struct MessageContent {
        content: String,
    }

    #[derive(Deserialize)]
    struct ChatResponse {
        choices: Vec<Choice>,
    }

    pub async fn get_inspirational_quote(
        api_key: &str,
        prompt: &str,
    ) -> Result<String, Box<dyn Error + Send + Sync>> {
        let client = Client::new();

        let body = ChatRequest {
            model: "gpt-3.5-turbo".to_string(),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
        };

        let res = client
            .post("https://api.openai.com/v1/chat/completions")
            .bearer_auth(api_key)
            .json(&body)
            .send()
            .await?;

        let data: ChatResponse = res.json().await?;
        let quote = &data.choices.get(0).unwrap().message.content;

        Ok(quote.trim().to_string())
    }

    pub async fn handle() -> StdResult<HashMap<String, String>, Box<dyn Error + Send + Sync>> {
        let topics_str = tokio::fs::read_to_string("/home/tombert/.config/sway/topics").await?;
        let topics: Vec<String> = topics_str.lines().map(|i| i.to_string()).collect();
        let mut rng = StdRng::from_entropy();
        let random_num = rng.gen_range(0..topics.len());
        let default_quote = "French Fry Dumpsters".to_string();
        let topic = topics.get(random_num).unwrap_or(&default_quote);
        let api_key = tokio::fs::read_to_string("/home/tombert/openai.key").await?;
        let api_key = api_key.trim();
        let prompt = format!(
            "Give me a very short inspirational quote about {} with a fictional author with a pun about {}",
            topic, topic
        );
        let quote = get_inspirational_quote(api_key, prompt.as_str()).await?;
        let out_map: HashMap<String, String> = [("quote", quote)]
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Ok(out_map)
    }
    pub fn render(i: &HashMap<String, String>) -> String {
        let error_text = "ERROR!".to_string();
        let quote = i.get(&"quote".to_string()).unwrap_or(&error_text);
        format!("{}", quote)
    }
}

pub mod battery {

    use std::collections::HashMap;
    use std::error::Error;
    use std::result::Result as StdResult;

    fn bat_status_icons(n: &str) -> &'static str {
        match n {
            "full" => "🟢",
            "charging" => "⚡",
            "notcharging" => "🔌",
            "discharging" => "🔋",
            _ => "",
        }
    }

    pub async fn handle() -> StdResult<HashMap<String, String>, Box<dyn Error + Send + Sync>> {
        let bat_path = "/sys/class/power_supply/BAT0";
        let cap_path = format!("{}/capacity", bat_path);
        let stat_path = format!("{}/status", bat_path);
        let cap_string = std::fs::read_to_string(cap_path)?.trim().replace("\"", "");
        let stat_string = std::fs::read_to_string(stat_path)?
            .trim()
            .to_lowercase()
            .replace(" ", "");

        let mut out_map = HashMap::new();
        out_map.insert("capacity".to_string(), cap_string.to_string());
        out_map.insert("status".to_string(), stat_string.to_string());
        Ok(out_map)
    }

    pub fn render(i: &HashMap<String, String>) -> String {
        let empty = "".to_string();
        let cap = i.get("capacity").unwrap_or(&empty);
        let stat = i.get("status").unwrap_or(&empty).as_str();
        format!("{} {}%", bat_status_icons(stat), cap)
    }
}

pub mod wifi {
    use std::collections::HashMap;
    use std::error::Error;
    use std::result::Result as StdResult;
    use tokio::process::Command;

    fn wifi_status_icons(n: &str) -> &'static str {
        match n {
            "connected" => "📶",
            "disconnected" => "❌",
            _ => "",
        }
    }

    pub async fn handle() -> StdResult<HashMap<String, String>, Box<dyn Error + Send + Sync>> {
        let wifi_cmd = Command::new("iw").arg("dev").output().await?;
        let s: Vec<String> = String::from_utf8_lossy(&wifi_cmd.stdout)
            .lines()
            .map(|s| s.trim().to_string())
            .collect();
        let interface: &str = s[5].split(" ").last().unwrap_or("");

        let connected_cmd = Command::new("iw")
            .arg(interface)
            .arg("link")
            .output()
            .await?;
        let s2 = String::from_utf8_lossy(&connected_cmd.stdout).find("Connected");

        let is_connected = match s2 {
            Some(_) => true,
            None => false,
        };

        let connect_status = if is_connected {
            "connected"
        } else {
            "disconnected"
        };

        let out_map: HashMap<String, String> = [("connect_status", connect_status)]
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        Ok(out_map)
    }

    pub fn render(i: &HashMap<String, String>) -> String {
        let empty = "".to_string();
        let connected = i.get("connect_status").unwrap_or(&empty);
        format!("{}", wifi_status_icons(&connected))
    }
}

pub mod volume {

    use std::collections::HashMap;
    use std::error::Error;
    use std::result::Result as StdResult;
    use tokio::process::Command;

    fn get_volume_icon(vol_level: i32, is_muted: bool) -> &'static str {
        let small_speaker_cutoff = 40;
        let mid_speaker_cutoff = 80;
        if is_muted {
            "🔇"
        } else if vol_level < small_speaker_cutoff {
            "🔈"
        } else if vol_level < mid_speaker_cutoff {
            "🔉"
        } else {
            "🔊"
        }
    }

    pub async fn handle() -> StdResult<HashMap<String, String>, Box<dyn Error + Send + Sync>> {
        //let SPACE = " ".to_string();
        let space = String::from(" ");
        let is_muted_cmd = Command::new("pactl")
            .arg("get-sink-mute")
            .arg("@DEFAULT_SINK@")
            .output()
            .await?;
        let vol_info_cmd = Command::new("pactl")
            .arg("get-sink-volume")
            .arg("@DEFAULT_SINK@")
            .output()
            .await?;
        let vol_info_str = String::from_utf8_lossy(&vol_info_cmd.stdout);
        let vol_info: Vec<String> = vol_info_str.split(&space).map(|i| i.to_string()).collect();
        let is_muted_raw_str = String::from_utf8_lossy(&is_muted_cmd.stdout);
        let is_muted_lower = is_muted_raw_str.to_lowercase();
        let is_muted_str = is_muted_lower.split(&space).map(|x| x.trim()).last();

        let is_muted = match is_muted_str {
            Some("yes") => "muted",
            _ => "not muted",
        }
        .to_string();

        let vol_level = if vol_info[5] == "/" {
            vol_info[4].to_string()
        } else {
            vol_info[5].to_string()
        }
        .replace("%", "");
        let out_map: HashMap<String, String> =
            [("volume_level", vol_level), ("is_muted", is_muted)]
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();

        Ok(out_map)
    }

    pub fn render(i: &HashMap<String, String>) -> String {
        let default_muted = "default_muted".to_string();
        let is_muted = i.get("is_muted").unwrap_or(&default_muted) == "muted";
        let default_vol = "50".to_string();
        let vol_level_str = i.get("volume_level").unwrap_or(&default_vol);

        let vol_level: i32 = vol_level_str.parse().unwrap_or(50);

        let icon = get_volume_icon(vol_level, is_muted);
        format!("{}{}%", icon, vol_level_str)
    }
}

pub mod date {
    use chrono::{Datelike, Local, Timelike};
    use std::collections::HashMap;
    use std::error::Error;
    use std::result::Result as StdResult;

    fn month_abbr(n: u32) -> &'static str {
        match n {
            1 => "Jan",
            2 => "Feb",
            3 => "Mar",
            4 => "Apr",
            5 => "May",
            6 => "Jun",
            7 => "Jul",
            8 => "Aug",
            9 => "Sep",
            10 => "Oct",
            11 => "Nov",
            12 => "Dec",
            _ => "???",
        }
    }

    pub async fn handle() -> StdResult<HashMap<String, String>, Box<dyn Error + Send + Sync>> {
        let now = Local::now();
        let weekday = now.weekday();
        let day = now.day();
        let month = month_abbr(now.month());
        let seconds = format!("{:02}", now.second());
        let mod_hour = if now.hour() % 12 == 0 {
            12
        } else {
            now.hour() % 12
        };

        let hour = format!("{:02}", mod_hour);
        let minutes = format!("{:02}", now.minute());
        let mut out_hash = HashMap::new();
        out_hash.insert("weekday".to_string(), weekday.to_string());
        out_hash.insert("day".to_string(), day.to_string());
        out_hash.insert("month".to_string(), month.to_string());
        out_hash.insert("seconds".to_string(), seconds.to_string());
        out_hash.insert("minutes".to_string(), minutes.to_string());
        out_hash.insert("hour".to_string(), hour.to_string());
        Ok(out_hash)
    }

    pub fn render(i: &HashMap<String, String>) -> String {
        static EMPTY: String = String::new();

        let hour = i.get("hour").unwrap_or(&EMPTY);
        let minute = i.get("minutes").unwrap_or(&EMPTY);
        let day = i.get("day").unwrap_or(&EMPTY);
        let month = i.get("month").unwrap_or(&EMPTY);
        let weekday = i.get("weekday").unwrap_or(&EMPTY);
        let seconds = i.get("seconds").unwrap_or(&EMPTY);
        format!(
            "{} {} {} {}:{} {}",
            weekday, month, day, hour, minute, seconds
        )
    }
}

#[derive(Serialize, Deserialize)]
pub struct Out {
    pub name: String,
    pub instance: String,
    pub full_text: String,
}

#[derive(Serialize, Deserialize)]
pub struct PersistConfig {
    pub path: String,
    pub buffer_size: i32,
}

#[derive(Serialize, Deserialize)]
pub struct ModuleConfig {
    pub name: String,
    pub ttl: u64,
    pub timeout: Option<u64>,
    pub display: Option<bool>,
}

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub poll_time: u64,
    pub default_timeout: u64,
    pub persist: PersistConfig,
    pub modules: Vec<ModuleConfig>,
}
