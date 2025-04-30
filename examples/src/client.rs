use tonic::{Request, Response, Status};

pub mod dict_example {
    tonic::include_proto!("dict_example");
}

use dict_example::*;

fn show_response<T>(r: Result<Response<T>, Status>)
where
    T: std::fmt::Debug,
{
    match r {
        Ok(reply) => println!("[GOOD] {:?}\n>>>", reply.get_ref()),
        Err(status) => println!("[FAIL] {:?}\n>>>", status.code()),
    }
}

#[tokio::main]
async fn main() {
    let mut client = dict_service_client::DictServiceClient::connect("http://127.0.0.1:50051")
        .await
        .unwrap();

    // query
    let request = Request::new(Key {
        key: String::from("pi"),
    });
    println!(">>> QUERY {:?}", request.get_ref());

    let response = client.query(request).await;
    show_response(response);

    // set
    let request = Request::new(Entry {
        key: String::from("pi"),
        value: 3.145926,
    });
    println!(">>> SET {:?}", request.get_ref());

    let response = client.set(request).await;
    show_response(response);

    // query
    let request = Request::new(Key {
        key: String::from("pi"),
    });
    println!(">>> QUERY {:?}", request.get_ref());

    let response = client.query(request).await;
    show_response(response);

    // sqrt
    let request = Request::new(Key {
        key: String::from("pi"),
    });
    println!(">>> SQRT {:?}", request.get_ref());

    let response = client.sqrt(request).await;
    show_response(response);

    // list_shart
    let request = Request::new(ListShardRequest {
        key: String::from("pi"),
    });
    println!(">>> LIST-SHARD {:?}", request.get_ref());

    let response = client.list_shard(request).await;
    show_response(response);

    // delete
    let request = Request::new(Key {
        key: String::from("pi"),
    });
    println!(">>> DELETE {:?}", request.get_ref());

    let response = client.delete(request).await;
    show_response(response);
}
