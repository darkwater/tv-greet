use core::pin::pin;

use futures::future::{Either, select};
use greetd_ipc::{Request, Response, codec::TokioCodec as _};
use tokio::{
    io::AsyncWriteExt as _,
    net::UnixStream,
    signal::unix::SignalKind,
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
};

pub struct Greetd {
    req_tx: UnboundedSender<Request>,
    res_rx: UnboundedReceiver<Response>,
}

impl Greetd {
    pub fn new() -> Self {
        let sock_addr =
            std::env::var("GREETD_SOCK").expect("GREETD_SOCK environment variable not set");

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        // do this blocking at the start so if we can't connect to greetd, we can exit before we do
        // anything graphical
        let sock = runtime
            .block_on(UnixStream::connect(sock_addr))
            .expect("Failed to connect to greetd socket");

        let (req_tx, req_rx) = tokio::sync::mpsc::unbounded_channel();
        let (res_tx, res_rx) = tokio::sync::mpsc::unbounded_channel();

        std::thread::spawn(move || {
            runtime.block_on(thread(sock, req_rx, res_tx));
        });

        Self { req_tx, res_rx }
    }

    pub fn send(&self, req: Request) {
        self.req_tx.send(req).unwrap();
    }

    pub fn recv(&mut self) -> Option<Response> {
        self.res_rx.try_recv().ok()
    }

    pub fn create_session(&self, username: &str) {
        self.send(Request::CreateSession {
            username: username.to_string(),
        });
    }

    pub fn start_session(&self, cmd: &[&str]) {
        self.send(Request::StartSession {
            cmd: cmd.iter().map(|s| s.to_string()).collect(),
            env: vec![],
        });
    }
}

async fn thread(
    sock: UnixStream,
    mut req_rx: UnboundedReceiver<Request>,
    res_tx: UnboundedSender<Response>,
) {
    let (mut greetd_rx, mut greetd_tx) = sock.into_split();

    tokio::spawn(async move {
        while let Some(req) = req_rx.recv().await {
            if let Err(e) = req.write_to(&mut greetd_tx).await {
                eprintln!("Failed to send request to greetd: {e}");
                break;
            }

            greetd_tx.flush().await.unwrap();
        }
    });

    let mut signal = tokio::signal::unix::signal(SignalKind::terminate()).unwrap();
    let mut signal = pin!(signal.recv());

    loop {
        match select(signal.as_mut(), Response::read_from(&mut greetd_rx)).await {
            Either::Left(_) => {
                break;
            }
            Either::Right((Ok(resp), _)) => res_tx.send(resp).unwrap(),
            Either::Right((Err(e), _)) => {
                panic!("Failed to read response from greetd: {e}");
            }
        }
    }
}
