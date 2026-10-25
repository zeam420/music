#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::{io::{Read, Write}, net::TcpListener, process::Command, time::{Duration, Instant}};
use tauri::{Emitter, LogicalSize, WebviewWindow};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, ShortcutState};

// One-shot local listener for the Spotify OAuth redirect (127.0.0.1:43019).
#[tauri::command]
async fn wait_oauth() -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let l = TcpListener::bind("127.0.0.1:43019").map_err(|e| e.to_string())?;
        l.set_nonblocking(true).ok();
        let t0 = Instant::now();
        loop {
            let mut s = match l.accept() {
                Ok((s, _)) => s,
                Err(_) if t0.elapsed().as_secs() < 180 => { std::thread::sleep(Duration::from_millis(150)); continue; }
                Err(e) => return Err(e.to_string()),
            };
            s.set_nonblocking(false).ok();
            s.set_read_timeout(Some(Duration::from_secs(3))).ok();
            let mut buf = [0u8; 4096];
            let n = s.read(&mut buf).unwrap_or(0);
            let req = String::from_utf8_lossy(&buf[..n]).to_string();
            let q = req.lines().next().unwrap_or("").split(' ').nth(1).unwrap_or("").split_once('?').map(|x| x.1.to_string()).unwrap_or_default();
            let body = "<body style='background:#0a0c16;color:#6fd6ff;font:22px sans-serif;display:grid;place-items:center;height:100vh'>OK - mozesz zamknac te karte / you can close this tab</body>";
            let _ = write!(s, "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body);
            if q.contains("code=") || q.contains("error=") { return Ok(q); }
        }
    }).await.map_err(|e| e.to_string())?
}

#[tauri::command]
fn open_url(url: String) { let _ = Command::new("rundll32").args(["url.dll,FileProtocolHandler", &url]).spawn(); }

#[tauri::command]
fn win(w: WebviewWindow, a: String) {
    match a.as_str() {
        "min" => { let _ = w.minimize(); }
        "max" => { if w.is_maximized().unwrap_or(false) { let _ = w.unmaximize(); } else { let _ = w.maximize(); } }
        _ => { let _ = w.close(); }
    }
}

#[tauri::command]
fn set_mini(w: WebviewWindow, mini: bool) {
    let _ = w.unmaximize();
    let _ = w.set_always_on_top(mini);
    let _ = w.set_size(LogicalSize::new(if mini { 430.0 } else { 1100.0 }, if mini { 126.0 } else { 720.0 }));
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .invoke_handler(tauri::generate_handler![wait_oauth, open_url, win, set_mini])
        .setup(|app| {
            app.global_shortcut().on_shortcuts(["MediaPlayPause", "MediaTrackNext", "MediaTrackPrevious"], |app, sc, ev| {
                if ev.state == ShortcutState::Pressed {
                    let k = match sc.key { Code::MediaPlayPause => "pp", Code::MediaTrackNext => "next", _ => "prev" };
                    let _ = app.emit("media", k);
                }
            })?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running app");
}
