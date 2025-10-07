use std::fs;
use std::fs::File;
use std::path::PathBuf;
use std::io::{self, Write, Read};
use std::time::{Instant, Duration};
use std::process::exit;

#[derive(Clone)]
struct SharpEntry {
    url: String,
    output: PathBuf,
}

struct Args {
    url: String,
    limit: String,
    out: PathBuf,
    nogui: bool,
    raw: bool,
    install: bool,
    minst: bool,
    sharp_entries: Vec<SharpEntry>,
}

fn parse_args() -> Args {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::System::Console::*;
        use windows_sys::Win32::Foundation::HANDLE;
        unsafe {
            let stdout: HANDLE = GetStdHandle(STD_OUTPUT_HANDLE);
            let mut mode: u32 = 0;
            if GetConsoleMode(stdout, &mut mode) != 0 {
                SetConsoleMode(stdout, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
            }
        }
    }

    let mut url = None;
    let mut limit = "infinity".to_string();
    let mut out = PathBuf::from(".");
    let mut nogui = false;
    let mut raw = false;
    let mut install = false;
    let mut minst = false;
    let mut sharp_entries: Vec<SharpEntry> = Vec::new();

    let mut iter = std::env::args().skip(1);
    let mut sharp_used = false;
    let mut install_used = false;

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--install" | "-i" => { url = iter.next(); install = true; install_used = true; }
            "--limit" | "-l" => limit = iter.next().unwrap_or("infinity".to_string()),
            "--output" | "-o" => out = PathBuf::from(iter.next().unwrap_or(".".to_string())),
            "--nogui" | "-ng" => nogui = true,
            "-raw" => raw = true,
            "--sharp" | "-s" => {
                let sharp_file = iter.next().unwrap_or_else(|| {
                    eprintln!("\x1b[31mError: --sharp requires a file path\x1b[0m"); exit(1);
                });
                minst = true; sharp_used = true;

                let content = fs::read_to_string(&sharp_file).unwrap_or_else(|_| {
                    eprintln!("\x1b[31mError: Failed to read sharp file: {}\x1b[0m", sharp_file); exit(1);
                });

                for line in content.lines() {
                    let line = line.trim();
                    if line.is_empty() || line.starts_with("sharp") || line.starts_with('{') || line.starts_with('}') { continue; }
                    let mut parts = line.split('"').map(|s| s.trim()).filter(|s| !s.is_empty());
                    let url_part = parts.next().unwrap_or_default().to_string();
                    let out_dir = parts.next().unwrap_or_default().to_string();

                    let mut final_url = url_part.clone();
                    if line.contains("-raw") {
                        if let Some(js) = github_to_jsdelivr(&url_part) { final_url = js; }
                    } else if !final_url.starts_with("http") {
                        final_url = format!("https://{}", final_url);
                    }

                    let output_path = PathBuf::from(&out_dir).join(fname(&url_part));
                    sharp_entries.push(SharpEntry { url: final_url, output: output_path });
                }
            }
            _ => { eprintln!("\x1b[33mWarning: Unknown argument: {}\x1b[0m", arg); exit(1); }
        }
    }

    // Проверка несовместимости
    if sharp_used && install_used {
        eprintln!("\x1b[31mError: --sharp is incompatible with --install\x1b[0m"); exit(1);
    }
    if sharp_used && raw {
        eprintln!("\x1b[31mError: --sharp mode cannot be combined with -raw (use per-entry -raw instead)\x1b[0m"); exit(1);
    }
    if sharp_used && out != PathBuf::from(".") {
        eprintln!("\x1b[31mError: --sharp mode does not support --output (outputs defined inside file)\x1b[0m"); exit(1);
    }

    if !install && !minst {
        eprintln!("\x1b[31mError: Either --install or --sharp must be specified\x1b[0m"); exit(1);
    }

    Args { url: url.unwrap_or_default(), limit, out, nogui, raw, install, minst, sharp_entries }
}



fn github_to_jsdelivr(url: &str) -> Option<String> {
    let s = if url.starts_with("http") { url.to_string() } else { format!("https://{}", url) };
    if s.contains("cdn.jsdelivr.net/gh/") || s.contains("raw.githubusercontent.com/") { return None; }
    let parts: Vec<&str> = s.split('/').collect();
    if parts.len() < 7 || parts[2] != "github.com" || parts[5] != "blob" { return None; }
    let user = parts[3]; let repo = parts[4]; let branch = parts[6];
    let path = if parts.len() > 7 { parts[7..].join("/") } else { String::new() };
    Some(if path.is_empty() {
        format!("https://cdn.jsdelivr.net/gh/{}/{}@{}/", user, repo, branch)
    } else {
        format!("https://cdn.jsdelivr.net/gh/{}/{}@{}/{}", user, repo, branch, path)
    })
}

fn fname(url: &str) -> String { url.rsplit('/').next().unwrap_or("file").to_string() }

fn lim_str(limit: &str) -> String {
    let s = limit.trim().to_uppercase();
    if s.is_empty() || s == "INFINITY" || s == "-1" { "Infinity".to_string() }
    else { format!("{}{}", &s[..s.len()-1], match s.chars().last().unwrap() {'K'=>" KB/s",'M'=>" MB/s",'G'=>" GB/s",'B'=>" B/s", _=>" B/s"}) }
}

fn eta(sec: f64) -> String {
    if sec < 60.0 { format!("{}s", sec.round() as u64) }
    else if sec < 3600.0 { format!("{}m", (sec/60.0).round() as u64) }
    else { format!("{}h", (sec/3600.0).round() as u64) }
}

#[cfg(target_os = "windows")]
fn hide() {
    use windows_sys::Win32::System::Console::GetConsoleWindow;
    use windows_sys::Win32::UI::WindowsAndMessaging::ShowWindow;
    unsafe { let hwnd = GetConsoleWindow(); if !hwnd.is_null() { ShowWindow(hwnd, 0); } }
}

fn total_size(url: &str, install: bool, minst: bool, _sharp_entries: &[SharpEntry]) -> Option<f64> {
    // Для каждого файла в Sharp multi-install берём размер отдельно
    if minst || install {
        let u = if url.starts_with("http") { url.to_string() } else { format!("https://{}", url) };
        if let Ok(resp) = ureq::head(&u).call() {
            if let Some(cl) = resp.header("Content-Length") {
                if let Ok(size) = cl.parse::<f64>() {
                    return Some(size / 1024.0 / 1024.0);
                }
            }
        }
        None
    } else {
        None
    }
}


fn download(url: &str, out: &PathBuf, limit: &str, _install: bool, _minst: bool, _entries: &[SharpEntry]) -> io::Result<()> {
    let resp = ureq::get(url).call()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let mut reader = resp.into_reader();
    let mut file = File::create(out)?;

    let rate: Option<u64> = match limit.trim().to_uppercase().as_str() {
        "INFINITY" | "-1" | "" => None,
        s if s.ends_with('K') => Some(s[..s.len()-1].parse::<u64>().unwrap_or(1)*1024),
        s if s.ends_with('M') => Some(s[..s.len()-1].parse::<u64>().unwrap_or(1)*1024*1024),
        s if s.ends_with('G') => Some(s[..s.len()-1].parse::<u64>().unwrap_or(1)*1024*1024*1024),
        s => Some(s.parse().unwrap_or(0)),
    };

    let mut buf = [0u8;1024];
    let mut downloaded = 0u64;
    let mut start = Instant::now();

    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 { break; }
        file.write_all(&buf[..n])?;

        if let Some(r) = rate {
            downloaded += n as u64;
            let elapsed = start.elapsed();
            if elapsed.as_secs_f64() < 1.0 && downloaded >= r {
                std::thread::sleep(Duration::from_secs_f64(1.0) - elapsed);
                downloaded = 0;
                start = Instant::now();
            } else if elapsed.as_secs_f64() >= 1.0 {
                downloaded = 0;
                start = Instant::now();
            }
        }
    }

    Ok(())
}

fn show_progress(out_path: &PathBuf, start: Instant, total: Option<f64>) -> io::Result<(f64, f64, String)> {
    let size = fs::metadata(out_path).map(|m| m.len() as f64 / 1024.0 / 1024.0).unwrap_or(0.0);
    let speed = size / start.elapsed().as_secs_f64().max(0.001);
    let eta_s = total.map_or("-".to_string(), |t| if speed > 0.0 { eta((t - size) / speed) } else { "-".to_string() });
    Ok((size, speed, eta_s))
}

fn remote_filename(url: &str) -> Option<String> { 
    if let Ok(resp) = ureq::head(url).call() {
        if let Some(disposition) = resp.header("Content-Disposition") {
            if let Some(pos) = disposition.find("filename=") {
                let fname = &disposition[pos + 9..];
                let fname = fname.trim_matches('"').trim();
                if !fname.is_empty() { return Some(fname.to_string()); }
            }
        }
    }
    None
}

fn download_with_ui(
    url: &str,
    out_path: &PathBuf,
    limit: &str,
    install: bool,
    minst: bool,
    sharp_entries: &[SharpEntry]
) -> io::Result<()> {
    let entries = if minst {
        sharp_entries.to_vec()
    } else {
        vec![SharpEntry { url: url.to_string(), output: out_path.clone() }]
    };

    let mut success_count = 0;
    let mut fail_count = 0;

    for mut entry in entries {
        // Создаём родительскую папку
        std::fs::create_dir_all(entry.output.parent().unwrap()).ok();

        // Определяем имя файла с сервера
        let server_name = remote_filename(&entry.url).unwrap_or_else(|| fname(&entry.url));
        entry.output = entry.output.parent().unwrap_or(&PathBuf::from(".")).join(server_name.clone());

        let total = total_size(&entry.url, install, minst, sharp_entries);
        let start = Instant::now();

        let out_clone = entry.output.clone();
        let url_clone = entry.url.clone();
        let limit_clone = limit.to_string();

        let handle = std::thread::spawn(move || {
            download(&url_clone, &out_clone, &limit_clone, install, minst, &[])
                .map(|_| true)
                .or_else(|_| Ok(false) as io::Result<bool>)
        });

        // Цикл отображения прогресса
        while !handle.is_finished() {
            let (size, speed, eta_s) = show_progress(&entry.output, start, total)?;
            print!(
                "\r\x1B[2K\x1B[?25l\x1b[33m{} - {:.3} MB / {:.3} MB | Speed: {:.2} MB/s | ETA: {}\x1b[0m",
                server_name,
                size,
                total.unwrap_or(0.0),
                speed,
                eta_s
            );
            io::stdout().flush()?;
            std::thread::sleep(Duration::from_millis(100));
        }

        let result = handle.join().unwrap_or(Ok(false))?;
        if result {
            success_count += 1;
            println!(
                "\r\x1B[2K\x1B[?25l\x1b[32m{} - {:.3} MB / {:.3} MB | Speed: {:.2} MB/s | ETA: {}\x1b[0m",
                server_name,
                total.unwrap_or(0.0),
                total.unwrap_or(0.0),
                0.0,
                "0s"
            );
        } else {
            fail_count += 1;
            println!("\r\x1B[2K\x1B[?25l\x1b[31m{} - download failed\x1b[0m", server_name);
        }
    }

    println!("\n\x1B[?25hDownload finished! Successful: {} | Failed: {}", success_count, fail_count);
    Ok(())
}


fn main() {
    let args = parse_args();

    if args.nogui { hide(); }

    let mut url = args.url.clone();
    if args.raw {
        if let Some(c) = github_to_jsdelivr(&url) {
            println!("Converted GitHub blob -> jsDelivr: {}", c);
            url = c;
        }
    }
    if !url.starts_with("http") {
        url = format!("https://{}", url);
    }

    if !args.out.exists() {
        fs::create_dir_all(&args.out).unwrap_or_else(|e| { eprintln!("Failed to create folder {}: {}", args.out.display(), e); exit(1); });
    }
    let out_path = if args.install { args.out.join(fname(&url)) } else { PathBuf::new() };

    // Очистка консоли
    if !args.nogui {
        if cfg!(target_os = "windows") {
            let _ = std::process::Command::new("cmd").args(["/C", "cls"]).status();
        } else {
            print!("\x1B[2J\x1B[1;1H");
        }
    }

    println!("Atmospect Web Service GUI");
    println!("(C) 2025 Forgotten Private Coalition. All Rights Reserved\n");
    println!("Mode: {}", if args.install { "Standart" } else { "Sharp multi-install" });
    println!("Limit: {}\n", lim_str(&args.limit));

    if let Err(e) = download_with_ui(&url, &out_path, &args.limit, args.install, args.minst, &args.sharp_entries) {
        eprintln!("Download failed: {}", e);
        exit(1);
    }
}
