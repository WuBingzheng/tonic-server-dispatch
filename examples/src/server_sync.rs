use tonic::{transport::Server, Status};
use std::collections::HashMap;

mod dict_example {
    tonic::include_proto!("dict_example");
}
use dict_example::*;

use dict_service_server::{DictService, DictServiceServer};

// STEP 1: Define the Service
tonic_server_dispatch::dispatch_service_sync! {
    DictService, // original service name
    key, // hash by this request field

    // service methods
    set(SetRequest) -> SetReply,
    get(Key) -> Value,
    delete(Key) -> Value,
}

// STEP 2: Implement the Service

// define your business context
#[derive(Clone, Default)]
struct DictCtx (HashMap<String, f64>);

// implement DispatchBackend for your context
impl DispatchBackend for DictCtx {
    fn set(&mut self, req: SetRequest) -> Result<SetReply, Status> {
        match self.0.insert(req.key, req.value) {
            Some(old_value) => Ok(SetReply {
                old_value: Some(old_value),
            }),
            None => Ok(SetReply {
                old_value: None,
            }),
        }
    }
    fn get(&mut self, req: Key) -> Result<Value, Status> {
        match self.0.get(&req.key) {
            Some(value) => Ok(Value { value: *value }),
            None => Err(Status::not_found(String::new())),
        }
    }
    fn delete(&mut self, req: Key) -> Result<Value, Status> {
        match self.0.remove(&req.key) {
            Some(value) => Ok(Value { value }),
            None => Err(Status::not_found(String::new())),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    // STEP 3: Start the Service
    //
    // This starts backend tasks and creates channels.
    // The requests are dispatched from network tasks to backend
    // tasks by the channels, and the response are sent back by
    // oneshot channels.
    let svc = start_simple_dispatch_backend(DictCtx::default(), 16, 10);

    let addr = "127.0.0.1:50051".parse()?;
    Server::builder()
        .add_service(DictServiceServer::new(svc))
        .serve(addr).await?;

    Ok(())
}
