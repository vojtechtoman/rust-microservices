use std::collections::HashMap;
use std::env;
use std::sync::{Arc, Mutex};

use dotenv::dotenv;
use log::debug;
use tonic::{Request, Response, Status, transport::Server};
use uuid::Uuid;


use todo_grpc_api::{CreateTodo, Empty, Todo, TodoList};
use todo_grpc_api::todo_service_server::{TodoService, TodoServiceServer};

mod todo_grpc_api {
    tonic::include_proto!("api");
}

const HOST_ENV_VAR: &str = "HOST";
const HOST_DEFAULT: &str = "127.0.0.1";
const PORT_ENV_VAR: &str = "PORT";
const PORT_DEFAULT: &str = "54321";

struct TodoStorage {
    todos: Arc<Mutex<HashMap<Uuid, Todo>>>,
}

impl TodoStorage {
    fn new() -> Self {
        Self { todos: Arc::new(Mutex::new(HashMap::new())) }
    }
}

#[tonic::async_trait]
impl TodoService for TodoStorage {
    async fn create(&self, request: Request<CreateTodo>) -> Result<Response<Todo>, Status> {
        debug!("PUT: {:?}", request);

        let uuid = Uuid::new_v4();
        let todo = Todo { id: uuid.to_string(), text: request.into_inner().text };
        let mut todos = self.todos.lock()
            .map_err(|e| Status::internal(format!("Failed to acquire mutex: {:}", e)))?;
        todos.insert(uuid, todo.clone());

        Ok(Response::new(todo))
    }

    async fn get_all(&self, _request: Request<Empty>) -> Result<Response<TodoList>, Status> {
        let todos = self.todos.lock()
            .map_err(|e| Status::internal(format!("Failed to acquire mutex: {:}", e)))?;
        let todos = TodoList { items: todos.values().map(|v| v.clone()).collect() };
        Ok(Response::new(todos))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    dotenv().ok();

    let host = env::var(HOST_ENV_VAR).unwrap_or(HOST_DEFAULT.to_string());
    let port = env::var(PORT_ENV_VAR).unwrap_or(PORT_DEFAULT.to_string());
    let addr = format!("{}:{}", host, port).parse()?;
    println!("Listening on {}", addr);

    Server::builder()
        .add_service(TodoServiceServer::new(TodoStorage::new()))
        .serve(addr)
        .await?;
    Ok(())
}