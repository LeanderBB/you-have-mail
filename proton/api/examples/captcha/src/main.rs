use proton_api_rs::clientv2::ping;
use proton_api_rs::domain::{HVCaptchaMessage, HumanVerificationLoginData, HumanVerificationType};
use proton_api_rs::{captcha_get, http, LoginError, Session};
use std::process::exit;

fn main() {
    env_logger::init();

    let user_email = std::env::var("PAPI_USER_EMAIL").unwrap();
    let user_password = std::env::var("PAPI_USER_PASSWORD").unwrap();
    let app_version = std::env::var("PAPI_APP_VERSION").unwrap();

    let client = http::ClientBuilder::new()
        .app_version(&app_version)
        .build::<http::ureq_client::UReqClient>()
        .unwrap();

    ping(&client).unwrap();

    let login_result = Session::login(&client, &user_email, &user_password, None, None);
    if let Err(LoginError::HumanVerificationRequired(hv)) = &login_result {
        let captcha_body = captcha_get(&client, &hv.token, true).unwrap();
        run_captcha(captcha_body, app_version, user_email, user_password);
    }

    if let Err(e) = &login_result {
        eprintln!("Got login error:{e}");
    }

    eprintln!("Human Verification request not triggered try again");
    return;
}

fn run_captcha(html: String, app_version: String, user: String, password: String) -> ! {
    std::fs::write("/tmp/captcha.html", &html).unwrap();
    use wry::{
        application::{
            event::{Event, StartCause, WindowEvent},
            event_loop::{ControlFlow, EventLoop},
            window::WindowBuilder,
        },
        webview::WebViewBuilder,
    };

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Proton API Captcha")
        .build(&event_loop)
        .unwrap();
    let _webview = WebViewBuilder::new(window)
        .unwrap()
        .with_html(html)
        //.with_url("http://127.0.0.1:8000/captcha.html")
        .unwrap()
        .with_devtools(true)
        .with_ipc_handler(move |_, req| match HVCaptchaMessage::new(&req) {
            Ok(m) => {
                println!("Got message {:?}", m);
                if let Some(token) = m.get_token() {
                    let client = http::ClientBuilder::new()
                        .app_version(&app_version)
                        .build::<http::ureq_client::UReqClient>()
                        .unwrap();

                    let login_result = Session::login(
                        &client,
                        &user,
                        &password,
                        Some(HumanVerificationLoginData {
                            hv_type: HumanVerificationType::Captcha,
                            token: token.to_string(),
                        }),
                        None,
                    );

                    if let Err(e) = login_result {
                        eprintln!("Captcha Err {e}");
                    } else {
                        println!("Log in success!!");
                        exit(0);
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to publish event{e}");
            }
        })
        .build()
        .unwrap();

    _webview
        .evaluate_script(
            "postMessageToParent = function(message) { window.ipc.postMessage(JSON.stringify(message), '*')}",
        )
        .unwrap();

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(StartCause::Init) => println!("Wry has started!"),
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                println!("Close requested");
                *control_flow = ControlFlow::Exit
            }
            _ => (),
        }
    });
}
