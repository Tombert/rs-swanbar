use clap::Parser;
use futures::FutureExt;
use serde_json::Value;
use std::error::Error;
use std::{collections::HashMap, fs::read_to_string, fs::write};
use tokio::io::{AsyncBufReadExt, BufReader, Stdin};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task::JoinHandle;
mod types;
mod handlers;
use futures::future::join_all;
use std::future::Future;
use std::pin::Pin;
use std::result::Result as StdResult;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use types::Meta;

macro_rules! boxed_handler {
    ($path:path) => {
        || -> Pin<Box<dyn Future<Output = _> + Send>> { Box::pin($path()) }
    };
}

fn get_handler(my_type: &str) -> (handlers::BoxedHandler, handlers::RenderFn) {
    match my_type {
        "date" => (boxed_handler!(handlers::date::handle), handlers::date::render),
        "battery" => (
            boxed_handler!(handlers::battery::handle),
            handlers::battery::render,
        ),
        "wifi" => (boxed_handler!(handlers::wifi::handle), handlers::wifi::render),
        "volume" => (boxed_handler!(handlers::volume::handle), handlers::volume::render),
        "quote" => (boxed_handler!(handlers::quote::handle), handlers::quote::render),
        "current" => (
            boxed_handler!(handlers::current_program::handle),
            handlers::current_program::render,
        ),
        "bgchange" => (
            boxed_handler!(handlers::bg_changer::handle),
            handlers::bg_changer::render,
        ),
        _ => (boxed_handler!(handlers::noop::handle), handlers::noop::render),
    }
}

async fn render(mut chan: Receiver<Vec<types::Out>>) {
    println!("{}", "{\"version\":1, \"click_events\":true}");
    println!("[");
    println!("[],");
    tokio::task::spawn(async move {
        while let Some(msg) = chan.recv().await {
            let out_json = serde_json::to_string(&msg);
            if let Ok(out) = out_json {
                println!("{},", out);
            }
        }
    });
}

fn get_mouse_handler(x: &str) -> handlers::MouseBoxedHandler {
    match x {
        "wifi" => boxed_handler!(handlers::wifi_click::click_handle),
        "volume" => boxed_handler!(handlers::volume_click::click_handle),
        _ => boxed_handler!(handlers::mouse_noop::click_handle),
    }
}

fn mouse_listener(chan: Sender<handlers::MouseBoxedHandler>, reader: BufReader<Stdin>) {
    let mut lines = reader.lines();

    tokio::task::spawn(async move {
        while let Ok(Some(line)) = lines.next_line().await {
            let line = if line.chars().nth(0).unwrap_or(' ') == ',' {
                line.trim_start_matches(',')
            } else {
                line.as_str()
            };
            //let line = line.trim_start_matches(',');
            if let Ok(value) = serde_json::from_str::<Value>(line) {
                let instance = &value["instance"];
                let inst = instance.as_str().unwrap_or("");
                let h = get_mouse_handler(inst);
                let _ = chan.send(h).await;
            }
        }
    });
}

fn write_state(mut chan: Receiver<HashMap<String, Meta>>, out_path: String, buffer_size: i32) {
    tokio::task::spawn(async move {
        let mut counter = 0;
        while let Some(msg) = chan.recv().await {
            if counter % buffer_size == 0 {
                let out_json = serde_json::to_string(&msg);
                match out_json {
                    Ok(x) => {
                        let _ = write(out_path.as_str(), x);
                    }
                    Err(_) => (),
                };
            }
            counter = (counter + 1) % buffer_size;
        }
    });
}

#[derive(Parser)]
#[command(name = "swaybar")]
#[command(author = "thomas@gebert.app")]
#[command(version = "1.0")]
#[command(about = "nada")]
pub struct Args {
    #[arg(short, long)]
    pub config: String,
}

fn possible_abort_task<T>(z: Option<JoinHandle<T>>) {
    if let Some(f) = z {
        f.abort();
    }

}

fn reset_state(state: &mut Meta) {
    state.is_processing = false;
    state.start_time = Duration::ZERO;
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> StdResult<(), Box<dyn Error>> {
    let stdin = tokio::io::stdin(); // 
    let reader = BufReader::new(stdin);
    let args = Args::parse();
    let path = args.config;
    let config_str = read_to_string(path)?;
    let config: types::Config = serde_json::from_str(config_str.as_str())?;
    let init_state_str = match read_to_string(config.persist.path.to_string()) {
        Ok(my_str) => my_str,
        Err(_) => String::from("{}"),
    };
    let mut state: HashMap<String, Meta> = serde_json::from_str(init_state_str.as_str())?;

    let poll_time = Duration::from_millis(config.poll_time);
    let (render_sender, render_receiver) = tokio::sync::mpsc::channel::<Vec<types::Out>>(5);
    render(render_receiver).await;

    let (state_sender, state_receiver) = tokio::sync::mpsc::channel::<HashMap<String, Meta>>(5);

    let (mouse_sender, mut mouse_receiver) =
        tokio::sync::mpsc::channel::<handlers::MouseBoxedHandler>(10);
    write_state(
        state_receiver,
        config.persist.path,
        config.persist.buffer_size,
    );
    let mut futures = HashMap::<String, JoinHandle<Option<HashMap<String, String>>>>::new();

    mouse_listener(mouse_sender, reader);
    loop {
        if let Some(Some(mouse_handle)) = mouse_receiver.recv().now_or_never() {
            tokio::spawn(async move {
                let _ = mouse_handle().await;
            });
        }
        let loop_begin = std::time::Instant::now();
        let mut futs = Vec::new();

        for module_config in &config.modules {
            let timeout_ms = module_config.timeout.unwrap_or(config.default_timeout);
            let timeout = Duration::from_millis(timeout_ms);
            let default = Meta {
                is_processing: false,
                start_time: Duration::ZERO,
                data: HashMap::new(),
            };
            let mut state = state.get(&module_config.name).unwrap_or(&default).clone();
            let ttl = Duration::from_millis(module_config.ttl);
            let name = module_config.name.clone();
            let display = module_config.display.unwrap_or(true);

            let old_fut = futures.remove(&name);

            let fin = async move {
                let name = name.as_str();
                let (handler, render) = get_handler(name);
                //let handler2 = get_handler(name);
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("Time went backwards");
                let expire_time = state.start_time + ttl;
                let mut my_f = if !state.is_processing && now > expire_time {
                    possible_abort_task(old_fut);

                    let fut = tokio::spawn(async move {
                        let f = handler().await;
                        match f {
                            Ok(ff) => Some(ff),
                            Err(_) => None,
                        }
                    });
                    state.is_processing = true;
                    state.start_time = now;
                    Some(fut)
                } else {
                    old_fut
                };

                let new_fut = match my_f.as_mut().and_then(|f| f.now_or_never()) {
                    Some(Ok(res)) => {
                        if let Some(res) = res {
                            state.data.extend(res);
                            state.is_processing = false;
                        }
                        None
                    }
                    _ => {
                        let elapsed = now.checked_sub(state.start_time).unwrap_or(Duration::ZERO);
                        match (state.is_processing, elapsed < timeout) {
                            (true, true) => my_f,
                            (true, false) => {
                                reset_state(&mut state);
                                possible_abort_task(my_f);
                                None
                            },
                            (false, _) => my_f
                        }
                    }
                };

                let out = display.then(|| render(&state.data));

                (name.to_string(), state, out, new_fut)
            };
            futs.push(fin);
        }

        let values = join_all(futs).await;

        let out_objs: Vec<types::Out> = values
            .into_iter()
            .filter_map(|(name, meta, out_str, new_fut)| {
                state.insert(name.clone(), meta);
                if let Some(f) = new_fut {
                    futures.insert(name.clone(), f);
                }

                out_str.map(|f| types::Out {
                    name: name.clone(),
                    instance: name,
                    full_text: f,
                })
            })
            .collect();

        let a1 = render_sender.send(out_objs);
        let a2 = state_sender.send(state.clone());
        let _ = tokio::join!(a1, a2);

        let elapsed = loop_begin.elapsed();
        let wait_time = poll_time.checked_sub(elapsed).unwrap_or(Duration::ZERO);
        tokio::time::sleep(wait_time).await;
    }
}
