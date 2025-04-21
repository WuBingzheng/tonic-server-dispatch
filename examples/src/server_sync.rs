use std::collections::HashMap;
use tonic::{transport::Server, Status};

mod dict_example {
    tonic::include_proto!("dict_example");
}
use dict_example::*;

use dict_service_server::{DictService, DictServiceServer};


// STEP 1: Define the Service
tonic_server_dispatch::dispatch_service_sync! {
    DictService, // original service name
    key: String, // hash by this request field

    [ // shard methods
        set(SetRequest) -> SetReply,
        delete(Key) -> Value,
    ],

    [ // mutable methods
        sqrt(Key) -> Value,
        add(AddRequest) -> Value,
    ],

    [ // readonly methods
        query(Key) -> Value,
    ]
}

// STEP 2: Implement the Service

// define your business context
#[derive(Clone, Default)]
struct DictCtx(HashMap<String, f64>);

// implement DispatchBackendShard for your context
impl DispatchBackendShard for DictCtx {
    type Item = f64;
    fn get_item(&self, key: &String) -> Result<&f64, Status> {
        self.0.get(key).ok_or(Status::not_found(String::new()))
    }
    fn get_item_mut(&mut self, key: &String) -> Result<&mut f64, Status> {
        self.0
            .get_mut(key)
            .ok_or(Status::not_found(String::new()))
    }

    // shard methods
    fn set(&mut self, req: SetRequest) -> Result<SetReply, Status> {
        match self.0.insert(req.key, req.value) {
            Some(old_value) => Ok(SetReply {
                old_value: Some(old_value),
            }),
            None => Ok(SetReply { old_value: None }),
        }
    }
    fn delete(&mut self, req: Key) -> Result<Value, Status> {
        match self.0.remove(&req.key) {
            Some(value) => Ok(Value { value }),
            None => Err(Status::not_found(String::new())),
        }
    }
}
// implement DispatchBackendItem for your context
impl DispatchBackendItem for f64 {
    fn sqrt(&mut self, _req: Key) -> Result<Value, Status> {
        *self *= *self;
        Ok(Value { value: *self })
    }
    fn add(&mut self, req: AddRequest) -> Result<Value, Status> {
        *self += req.value;
        Ok(Value { value: *self })
    }
    fn query(&self, _req: Key) -> Result<Value, Status> {
        Ok(Value { value: *self })
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
        .serve(addr)
        .await?;

    Ok(())
}
