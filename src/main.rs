#[macro_use]
extern crate rocket;

use imbue::{DataPoint, ImbueContext};
use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct ImbueRequest {
    dataset: Vec<DataPoint>,
    strategy: ImbueStrategy,
}

impl ImbueRequest {
    fn new(dataset: Vec<DataPoint>, strategy: ImbueStrategy) -> Self {
        ImbueRequest { dataset, strategy }
    }
}

impl From<ImbueRequest> for ImbueContext {
    fn from(request: ImbueRequest) -> Self {
        ImbueContext::new(request.dataset)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct ImbueResponse {
    dataset: Vec<DataPoint>,
}

impl ImbueResponse {
    fn new(dataset: Vec<DataPoint>) -> Self {
        ImbueResponse { dataset }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde", rename_all = "snake_case")]
enum ImbueStrategy {
    Average,
    Zeroed,
    LastKnown,
}

#[post("/imbue", data = "<request>", format = "json")]
fn imbue_data(request: Json<ImbueRequest>) -> Json<ImbueResponse> {
    let imbue = match request.strategy {
        ImbueStrategy::Average => imbue::average_imbue,
        ImbueStrategy::Zeroed => imbue::zeroed_imbue,
        ImbueStrategy::LastKnown => imbue::last_known_imbue,
    };
    let context = &ImbueContext::from(request.0);
    let imbued_dataset = imbue(context);

    Json(ImbueResponse::new(imbued_dataset))
}

// Will need this later https://cprimozic.net/blog/rust-rocket-cloud-run/#deploying
#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![imbue_data])
}

#[cfg(test)]
mod server_tests {
    use imbue::{DataPoint, ImbueContext};
    use rocket::http::Status;
    use rocket::local::blocking::Client;
    use rocket::serde::json::Json;

    use crate::{ImbueRequest, ImbueResponse, ImbueStrategy};

    use super::rocket;

    #[test]
    fn test_average_imbue() {
        let client = Client::tracked(rocket()).expect("Valid rocket instance required");
        let body = ImbueRequest::new(
            vec![
                DataPoint::new(1.0, 1.0),
                DataPoint::new(3.0, 3.0),
                DataPoint::new(5.0, 5.0),
            ],
            ImbueStrategy::Average,
        );
        let response = client.post("/imbue").json(&body).dispatch();
        assert_eq!(response.status(), Status::Ok);

        let result = response.into_json::<ImbueResponse>().unwrap();
        let expected_result = vec![DataPoint::new(2.0, 2.0), DataPoint::new(4.0, 4.0)];
        assert_eq!(result.dataset, expected_result)
    }

    #[test]
    fn test_zeroed_imbue() {
        let client = Client::tracked(rocket()).expect("Valid rocket instance required");
        let body = ImbueRequest::new(
            vec![
                DataPoint::new(1.0, 1.0),
                DataPoint::new(3.0, 3.0),
                DataPoint::new(5.0, 5.0),
            ],
            ImbueStrategy::Zeroed,
        );
        let response = client.post("/imbue").json(&body).dispatch();
        assert_eq!(response.status(), Status::Ok);

        let result = response.into_json::<ImbueResponse>().unwrap();
        let expected_result = vec![DataPoint::new(2.0, 0.0), DataPoint::new(4.0, 0.0)];
        assert_eq!(result.dataset, expected_result)
    }

    #[test]
    fn test_last_known_imbue() {
        let client = Client::tracked(rocket()).expect("Valid rocket instance required");
        let body = ImbueRequest::new(
            vec![
                DataPoint::new(1.0, 1.0),
                DataPoint::new(3.0, 3.0),
                DataPoint::new(5.0, 5.0),
            ],
            ImbueStrategy::LastKnown,
        );
        let response = client.post("/imbue").json(&body).dispatch();
        assert_eq!(response.status(), Status::Ok);

        let result = response.into_json::<ImbueResponse>().unwrap();
        let expected_result = vec![DataPoint::new(2.0, 1.0), DataPoint::new(4.0, 3.0)];
        assert_eq!(result.dataset, expected_result)
    }
}
