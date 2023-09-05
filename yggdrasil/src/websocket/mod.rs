use axum::{
    extract::{ws::WebSocket, WebSocketUpgrade},
    response::{Html, Response},
    routing::get,
    Router,
};
use miette::Result;
use tyr::{
    prelude::*,
    tasks::{AsyncDispatcher, Task},
};

pub struct WebsocketModule;

impl Module for WebsocketModule {
    fn initialize(self, app: App) -> Result<App> {
        app.add_startup_system(init_server)
    }
}

pub fn init_server(storage: &mut Storage) -> Result<()> {
    let guard = storage.get::<AsyncDispatcher>().unwrap().read().unwrap();
    let dispatcher: &AsyncDispatcher = guard.downcast_ref().unwrap();

    let mut dummy_task = Task::new();

    let app = Router::new()
        .route("/", get(hello_world))
        .route("/ws", get(handler));

    dispatcher.try_dispatch(&mut dummy_task, async {
        axum::Server::bind(&"127.0.0.1:3000".parse().unwrap())
            .serve(app.into_make_service())
            .await
            .unwrap();
    })?;

    Ok(())
}

async fn handler(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    while let Some(msg) = socket.recv().await {
        let msg = if let Ok(msg) = msg {
            msg
        } else {
            tracing::info!("{socket:?} disconnected: Failed to receive message");
            return;
        };

        if socket.send(msg).await.is_err() {
            tracing::info!("{socket:?} disconnected: Failed to send message");
            return;
        }
    }
}

async fn hello_world() -> Html<&'static str> {
    Html("<h1>Hello world!</h1>")
}
