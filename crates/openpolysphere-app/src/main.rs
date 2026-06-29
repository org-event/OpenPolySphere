mod env;
#[cfg(target_os = "macos")]
mod macos_mic;

use std::fs::OpenOptions;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use wry::WebViewBuilder;

const SERVER_URL: &str = "http://127.0.0.1:5050/";
const SERVER_WAIT: Duration = Duration::from_secs(30);

struct ServerGuard(Option<Child>);

impl ServerGuard {
    fn spawn() -> std::io::Result<Self> {
        env::apply_packaged_env();
        #[cfg(target_os = "macos")]
        if let Err(e) = macos_mic::ensure_microphone_access() {
            log::warn!("Microphone access: {e:#}");
        }
        let translator = env::translator_exe();
        let mut cmd = Command::new(&translator);
        cmd.arg("serve").envs(std::env::vars()).stdin(Stdio::null());
        if env::is_packaged() {
            let log_path = env::user_data_dir().join("translator.log");
            let _ = std::fs::create_dir_all(log_path.parent().unwrap_or(std::path::Path::new(".")));
            let log_file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_path)?;
            cmd.stdout(Stdio::null()).stderr(Stdio::from(log_file));
        } else {
            cmd.stdout(Stdio::null()).stderr(Stdio::null());
        }
        let child = cmd.spawn()?;
        Ok(Self(Some(child)))
    }

    fn stop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for ServerGuard {
    fn drop(&mut self) {
        self.stop();
    }
}

fn run_translator_cli(args: &[String]) -> ! {
    env::apply_packaged_env();
    let translator = env::translator_exe();
    let status = Command::new(&translator)
        .args(args)
        .envs(std::env::vars())
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .unwrap_or_else(|e| {
            eprintln!("failed to run {}: {e}", translator.display());
            std::process::exit(127);
        });
    std::process::exit(status.code().unwrap_or(1));
}

fn wait_for_server() -> bool {
    let deadline = Instant::now() + SERVER_WAIT;
    while Instant::now() < deadline {
        if server_responding() {
            return true;
        }
        thread::sleep(Duration::from_millis(250));
    }
    false
}

fn server_responding() -> bool {
    use std::net::{SocketAddr, TcpStream};
    let addr: SocketAddr = "127.0.0.1:5050".parse().unwrap();
    TcpStream::connect_timeout(&addr, Duration::from_millis(400)).is_ok()
}

fn run_gui() {
    let mut server = match ServerGuard::spawn() {
        Ok(s) => s,
        Err(e) => {
            eprintln!("could not start translator server: {e}");
            std::process::exit(1);
        }
    };

    if !wait_for_server() {
        server.stop();
        eprintln!("OpenPolySphere server did not start on {SERVER_URL}");
        std::process::exit(1);
    }

    let event_loop = EventLoop::new();
    let version = env!("OPENPOLYSPHERE_VERSION");
    let window = WindowBuilder::new()
        .with_title(format!("OpenPolySphere {version}"))
        .with_inner_size(tao::dpi::LogicalSize::new(1180.0, 820.0))
        .with_min_inner_size(tao::dpi::LogicalSize::new(960.0, 640.0))
        .build(&event_loop)
        .unwrap_or_else(|e| {
            eprintln!("could not create window: {e}");
            std::process::exit(1);
        });

    let builder = WebViewBuilder::new().with_url(SERVER_URL);

    #[cfg(not(target_os = "linux"))]
    let _webview = builder.build(&window).unwrap_or_else(|e| {
        eprintln!("could not create webview: {e}");
        std::process::exit(1);
    });
    #[cfg(target_os = "linux")]
    let _webview = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        builder
            .build_gtk(window.default_vbox().unwrap())
            .unwrap_or_else(|e| {
                eprintln!("could not create webview: {e}");
                std::process::exit(1);
            })
    };

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        if let Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } = event
        {
            server.stop();
            *control_flow = ControlFlow::Exit;
        }
    });
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.is_empty() {
        run_translator_cli(&args);
    }
    run_gui();
}
