
use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use std::result::Result as StdResult;
use std::collections::HashMap;


pub type BoxedHandler = fn() -> Pin<
    Box<
        dyn Future<Output = StdResult<HashMap<String, String>, Box<dyn Error + Send + Sync>>>
            + Send,
    >,
>;

pub type MouseBoxedHandler =
    fn() -> Pin<Box<dyn Future<Output = StdResult<(), Box<dyn Error + Send + Sync>>> + Send>>;

pub type RenderFn = fn(&HashMap<String, String>) -> String;


pub mod volume_click {

    use std::error::Error;
    use std::result::Result as StdResult;
    use tokio::process::Command;
    pub async fn click_handle() -> StdResult<(), Box<dyn Error + Send + Sync>> {
        Command::new("pavucontrol").output().await?;
        Ok(())
    }
}

pub mod prog_click {
    use std::error::Error;
    use std::result::Result as StdResult;
    use tokio::process::Command;
    pub async fn click_handle() -> StdResult<(), Box<dyn Error + Send + Sync>> { 
        Command::new("foot")
            .arg("sh")
            .arg("-c")
            .arg("/home/tombert/.config/sway/prog-select")
            .output()
            .await?;
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
    use std::error::Error;
    use std::result::Result as StdResult;
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
        out_hash.insert(String::from(""), String::from(""));
        Ok(out_hash)
    }
    pub fn render(_i: &HashMap<String, String>) -> String {
        String::from("")
    }
}

pub mod noop {

    use std::collections::HashMap;
    use std::error::Error;
    use std::result::Result as StdResult;

    pub async fn handle() -> StdResult<HashMap<String, String>, Box<dyn Error + Send + Sync>> {
        let out_hash = HashMap::from([(String::from(""), String::from(""))]);
        Ok(out_hash)
    }
    pub fn render(_i: &HashMap<String, String>) -> String {
        String::from("")
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
            HashMap::from([(String::from("out"), String::from(name))])
        } else {
            HashMap::from([(String::from("out"), String::from("nothing"))])
        };

        Ok(om)
    }
    pub fn render(i: &HashMap<String, String>) -> String {
        format!(
            "{}",
            i.get(&String::from("out")).unwrap_or(&String::from("nada"))
        )
    }
}

pub mod quote {
    use rand::Rng;
    use rand::SeedableRng;
    use rand::rngs::StdRng;
    use std::fs::File;
    use std::io::{BufRead, BufReader};
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
            model: String::from("gpt-3.5-turbo"),
            messages: vec![Message {
                role: String::from("user"),
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
    fn pick_random_line(path: &str) -> Option<String> {
        let file = File::open(path).ok()?;
        let reader = BufReader::new(file);
        let mut rng = rand::thread_rng();

        let mut selected = None;
        for (i, line) in reader.lines().enumerate() {
            let line = line.ok()?;
            if rng.gen_range(0..=i) == 0 {
                selected = Some(line);
            }
        }

        selected
    }

    pub async fn handle() -> StdResult<HashMap<String, String>, Box<dyn Error + Send + Sync>> {
        //let topics_str = tokio::fs::read_to_string().await?;
        let topic = pick_random_line("/home/tombert/.config/sway/topics").unwrap();
        //let topics: Vec<String> = topics_str.lines().map(|i| i.to_string()).collect();
        //let default_quote = String::from("French Fry Dumpsters");
        //let topic = topics.get(random_num).unwrap_or(&default_quote);
        let api_key = tokio::fs::read_to_string("/home/tombert/openai.key").await?;
        let api_key = api_key.trim();
        let prompt = format!(
            "Give me a very short inspirational quote about {} with a fictional author with a pun about {}",
            topic, topic
        );
        let quote = get_inspirational_quote(api_key, prompt.as_str()).await?;
        let out_map: HashMap<String, String> = HashMap::from([(String::from("quote"), quote.to_string())]);
        Ok(out_map)
    }
    pub fn render(i: &HashMap<String, String>) -> String {
        let error_text = String::from("ERROR!");
        let quote = i.get(&String::from("quote")).unwrap_or(&error_text);
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

        let out_map = HashMap::from(
            [(String::from("capacity"), cap_string.to_string()),
            (String::from("status"), stat_string.to_string()) ]);

        Ok(out_map)
    }

    pub fn render(i: &HashMap<String, String>) -> String {
        let empty = String::from("");
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

        let out_map = HashMap::from([(String::from("connect_status"), String::from(connect_status))]);

        Ok(out_map)
    }

    pub fn render(i: &HashMap<String, String>) -> String {
        let empty = String::from("");
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
        let out_map =
            HashMap::from(
                [(String::from("volume_level"), String::from(vol_level)), 
                (String::from("is_muted"), String::from(is_muted))]);

        Ok(out_map)
    }

    pub fn render(i: &HashMap<String, String>) -> String {
        let default_muted = String::from("default_muted");
        let is_muted = i.get("is_muted").unwrap_or(&default_muted) == "muted";
        let default_vol = String::from("50");
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

        let out_hash = 
            HashMap::from(
                [(String::from("weekday"), weekday.to_string()),
                (String::from("day"), day.to_string()),
                (String::from("month"), month.to_string()),
                (String::from("seconds"), seconds.to_string()),
                (String::from("minutes"), minutes.to_string()),
                (String::from("hour"), hour.to_string())]);

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
