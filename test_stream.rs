use silvapi::http::HttpClient;
use silvapi::models::{ApiRequest, Auth, Body};

fn main() {
    let client = HttpClient::new();
    let req = ApiRequest {
        id: "1".into(),
        name: "test".into(),
        method: "GET".into(),
        url: "http://127.0.0.1:8080/".into(),
        params: vec![],
        headers: vec![],
        auth: Auth { auth_type: silvapi::models::AuthType::None, ..Default::default() },
        body: Body { body_type: silvapi::models::BodyType::None, ..Default::default() },
    };
    
    let res = client.execute(
        &req,
        "http://127.0.0.1:8080/",
        |_| { println!("Start!"); },
        |chunk| { println!("Chunk: {:?}", String::from_utf8_lossy(chunk)); }
    );
    println!("Done");
}
