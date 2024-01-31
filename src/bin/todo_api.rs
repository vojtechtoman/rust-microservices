use std::env;
use std::io::ErrorKind;

use actix_web::web::{Data, Json};
use actix_web::{get, post, App, HttpResponse, HttpServer, Responder};
use dotenv::dotenv;
use log::error;
use serde::{Deserialize, Serialize};
use tonic::transport::Channel;

use todo_grpc_api::todo_service_client::TodoServiceClient;
use todo_grpc_api::Empty;

mod todo_grpc_api {
    tonic::include_proto!("api");
}

const HOST_ENV_VAR: &str = "HOST";
const HOST_DEFAULT: &str = "127.0.0.1";
const PORT_ENV_VAR: &str = "PORT";
const PORT_DEFAULT: &str = "8088";
const TODO_SERVICE_ENDPOINT_ENV_VAR: &str = "TODO_SERVICE_ENDPOINT";

#[derive(Serialize, Deserialize)]
pub struct CreateTodo {
    pub text: String,
}

#[derive(Serialize)]
pub struct Todo {
    pub id: String,
    pub text: String,
}

#[derive(Serialize)]
pub struct TodoList {
    pub items: Vec<Todo>,
}

#[post("/api/todo")]
async fn create_todo(channel: Data<Channel>, todo: Json<CreateTodo>) -> impl Responder {
    let mut client = TodoServiceClient::new(channel.get_ref().to_owned());
    match client
        .create(todo_grpc_api::CreateTodo {
            text: todo.text.clone(),
        })
        .await
    {
        Ok(created) => {
            let created = created.into_inner();
            let todo = Todo {
                id: created.id.clone(),
                text: created.text.clone(),
            };
            let json = HttpResponse::Created().json(todo);
            json
        }
        Err(err) => {
            error!("gRPC call failed: {}", err);
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[get("/api/todos")]
async fn get_todos(channel: Data<Channel>) -> impl Responder {
    let mut client = TodoServiceClient::new(channel.get_ref().to_owned());
    match client.get_all(Empty::default()).await {
        Ok(todos) => HttpResponse::Ok().json(TodoList {
            items: todos
                .into_inner()
                .items
                .into_iter()
                .map(|t| Todo {
                    id: t.id,
                    text: t.text,
                })
                .collect(),
        }),
        Err(err) => {
            error!("gRPC call failed: {}", err);
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    dotenv().ok();

    let todo_service_endpoint = env::var(TODO_SERVICE_ENDPOINT_ENV_VAR).map_err(|e| {
        error!(
            "Environment variable {} not set",
            TODO_SERVICE_ENDPOINT_ENV_VAR
        );
        std::io::Error::new(ErrorKind::Other, e)
    })?;

    let channel: Channel = match tonic::transport::Endpoint::new(todo_service_endpoint) {
        Err(err) => {
            error!("Failed to create endpoint: {}", err);
            Err(err)
        }
        Ok(e) => match e.connect().await {
            Ok(c) => Ok(c),
            Err(err) => {
                error!("Failed to connect to gRPC service: {}", err);
                Err(err)
            }
        },
    }
    .map_err(|e| std::io::Error::new(ErrorKind::Other, e))?;

    let host = env::var(HOST_ENV_VAR).unwrap_or(HOST_DEFAULT.to_string());
    let port = env::var(PORT_ENV_VAR).unwrap_or(PORT_DEFAULT.to_string());
    let addr = format!("{}:{}", host, port);
    println!("Listening on {}", addr);

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(channel.clone()))
            .service(get_todos)
            .service(create_todo)
    })
    .bind(addr)?
    .run()
    .await
}
