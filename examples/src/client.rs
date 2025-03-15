use tonic::{Request, Response, Status};

pub mod dict_example {
    tonic::include_proto!("dict_example");
}

use dict_example::*;

fn show_response<T>(r: Result<Response<T>, Status>)
    where T: std::fmt::Debug
{
    match r {
        Ok(reply) => println!("[GOOD] {:?}\n>>>", reply.get_ref()),
        Err(status) => println!("[FAIL] {:?}\n>>>", status.code()),
    }
}

#[tokio::main]
async fn main() {
    let mut client = dict_service_client::DictServiceClient::connect("http://127.0.0.1:50051").await.unwrap();

    // get
    let request = Request::new(Key {
        key: String::from("pi"),
    });
    println!(">>> GET {:?}", request.get_ref());

    let response = client.get(request).await;
    show_response(response);

    // set
    let request = Request::new(SetRequest {
        key: String::from("pi"),
        value: 3.145926,
    });
    println!(">>> SET {:?}", request.get_ref());

    let response = client.set(request).await;
    show_response(response);

    // get
    let request = Request::new(Key {
        key: String::from("pi"),
    });
    println!(">>> GET {:?}", request.get_ref());

    let response = client.get(request).await;
    show_response(response);

    // delete
    let request = Request::new(Key {
        key: String::from("pi"),
    });
    println!(">>> DELETE {:?}", request.get_ref());

    let response = client.delete(request).await;
    show_response(response);
}
