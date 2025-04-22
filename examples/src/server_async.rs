use std::collections::HashMap;
use tonic::{transport::Server, Status};

mod dict_example {
    tonic::include_proto!("dict_example");
}
use dict_example::*;

use dict_service_server::{DictService, DictServiceServer};


// STEP 1: Define the Service
//
// This builds the mapping relationship between tonic
// network tasks and your business tasks.
tonic_server_dispatch::dispatch_service_async! {
    DictService, // original service name
    key: String, // hash by this request field

    [ // shard methods, work on the shard. E.g. create/remove item on the shard.
        set(SetRequest) -> SetReply,
        delete(Key) -> Value,
    ],

    [ // mutable methods, work on mutable item. E.g. update item.
        sqrt(Key) -> Value,
        add(AddRequest) -> Value,
    ],

    [ // readonly methods, work on readonly item. E.g. query item.
        query(Key) -> Value,
    ]
}

// STEP 2: Implement the Service

// 2.1 define your business context for each shard
#[derive(Clone, Default)]
struct DictCtx(HashMap<String, f64>);

// 2.2 implement DispatchBackendShard for your shard context
impl DispatchBackendShard for DictCtx {
    // part-1: the associated type and get_item/get_item_mut methods.
    type Item = f64;
    fn get_item(&self, key: &String) -> Result<&f64, Status> {
        self.0.get(key).ok_or(Status::not_found(String::new()))
    }
    fn get_item_mut(&mut self, key: &String) -> Result<&mut f64, Status> {
        self.0
            .get_mut(key)
            .ok_or(Status::not_found(String::new()))
    }

    // part-2: shard methods. The `self` here points to each shard.
    async fn set(&mut self, req: SetRequest) -> Result<SetReply, Status> {
        match self.0.insert(req.key, req.value) {
            Some(old_value) => Ok(SetReply {
                old_value: Some(old_value),
            }),
            None => Ok(SetReply { old_value: None }),
        }
    }
    async fn delete(&mut self, req: Key) -> Result<Value, Status> {
        match self.0.remove(&req.key) {
            Some(value) => Ok(Value { value }),
            None => Err(Status::not_found(String::new())),
        }
    }
}

// 2.3 implement DispatchBackendItem for your item
impl DispatchBackendItem for f64 {
    // mutable methods. The `self` here points to each item.
    async fn sqrt(&mut self, _req: Key) -> Result<Value, Status> {
        *self *= *self;
        Ok(Value { value: *self })
    }
    async fn add(&mut self, req: AddRequest) -> Result<Value, Status> {
        *self += req.value;
        Ok(Value { value: *self })
    }

    // readonly methods. The `self` here points to each item.
    async fn query(&self, _req: Key) -> Result<Value, Status> {
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
    //
    // As the function's name suggests, it just starts the simple
    // kind of backend task, which just listen on the request channel.
    // If you want more complex backend task (e.g. listen on another
    // channel too), you have to create tasks and channels youself.
    // However, the implementation of this function can also be used
    // as your reference.
    let svc = start_simple_dispatch_backend(DictCtx::default(), 16, 10);

    let addr = "127.0.0.1:50051".parse()?;
    Server::builder()
        .add_service(DictServiceServer::new(svc))
        .serve(addr).await?;

    Ok(())
}
