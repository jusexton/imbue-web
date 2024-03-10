#[macro_use]
extern crate rocket;

use imbue::{DataPoint, ImbueContext};
use rocket::serde::json::Json;
use rocket::serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct ImbueRequest {
    dataset: Vec<DataPointWrapper>,
    strategy: ImbueStrategy,
}

impl From<ImbueRequest> for ImbueContext {
    fn from(request: ImbueRequest) -> Self {
        let mapped = request
            .dataset
            .into_iter()
            .map(|point| DataPoint::new(point.x, point.y))
            .collect();
        ImbueContext::new(mapped)
    }
}

// Tech Debt:
//
// The imbue DataPoint struct can not be serialized due to the orphan rule.
// To get around this, we create our own "version" of the struct and map between the two
// this way we get full control of the data.
//
// Serde offers a solution to this problem but it does not seem to be working. Will look into this later.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(crate = "rocket::serde")]
struct DataPointWrapper {
    pub x: f64,
    pub y: f64,
}

impl DataPointWrapper {
    fn new(x: f64, y: f64) -> Self {
        DataPointWrapper { x, y }
    }
}

impl From<DataPoint> for DataPointWrapper {
    fn from(point: DataPoint) -> Self {
        DataPointWrapper::new(point.x, point.y)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "rocket::serde")]
struct ImbueResponse {
    dataset: Vec<DataPointWrapper>,
}

impl ImbueResponse {
    fn new(dataset: Vec<DataPointWrapper>) -> Self {
        ImbueResponse { dataset }
    }

    fn from_imbued(dataset: Vec<DataPoint>) -> Self {
        let mapped = dataset.into_iter().map(DataPointWrapper::from).collect();
        ImbueResponse::new(mapped)
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
        ImbueStrategy::Average => imbue::average,
        ImbueStrategy::Zeroed => imbue::zeroed,
        ImbueStrategy::LastKnown => imbue::last_known,
    };
    let context = &ImbueContext::from(request.0);
    let imbued_dataset = imbue(context);

    Json(ImbueResponse::from_imbued(imbued_dataset))
}

// Will need this later https://cprimozic.net/blog/rust-rocket-cloud-run/#deploying
#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![imbue_data])
}

#[cfg(test)]
mod server_tests {
    use rocket::http::Status;
    use rocket::local::blocking::Client;

    use crate::{DataPointWrapper, ImbueRequest, ImbueResponse, ImbueStrategy};

    use super::rocket;

    #[test]
    fn test_average_imbue() {
        let client = Client::tracked(rocket()).expect("Valid rocket instance required");
        let body = {
            let dataset = vec![
                DataPointWrapper::new(1.0, 1.0),
                DataPointWrapper::new(3.0, 3.0),
                DataPointWrapper::new(5.0, 5.0),
            ];
            let strategy = ImbueStrategy::Average;
            ImbueRequest { dataset, strategy }
        };
        let response = client.post("/imbue").json(&body).dispatch();
        assert_eq!(response.status(), Status::Ok);

        let result = response.into_json::<ImbueResponse>().unwrap();
        let expected_result = vec![
            DataPointWrapper::new(2.0, 2.0),
            DataPointWrapper::new(4.0, 4.0),
        ];
        assert_eq!(result.dataset, expected_result)
    }

    #[test]
    fn test_zeroed_imbue() {
        let client = Client::tracked(rocket()).expect("Valid rocket instance required");
        let body = {
            let dataset = vec![
                DataPointWrapper::new(1.0, 1.0),
                DataPointWrapper::new(3.0, 3.0),
                DataPointWrapper::new(5.0, 5.0),
            ];
            let strategy = ImbueStrategy::Zeroed;
            ImbueRequest { dataset, strategy }
        };
        let response = client.post("/imbue").json(&body).dispatch();
        assert_eq!(response.status(), Status::Ok);

        let result = response.into_json::<ImbueResponse>().unwrap();
        let expected_result = vec![
            DataPointWrapper::new(2.0, 0.0),
            DataPointWrapper::new(4.0, 0.0),
        ];
        assert_eq!(result.dataset, expected_result)
    }

    #[test]
    fn test_last_known_imbue() {
        let client = Client::tracked(rocket()).expect("Valid rocket instance required");
        let body = {
            let dataset = vec![
                DataPointWrapper::new(1.0, 1.0),
                DataPointWrapper::new(3.0, 3.0),
                DataPointWrapper::new(5.0, 5.0),
            ];
            let strategy = ImbueStrategy::LastKnown;
            ImbueRequest { dataset, strategy }
        };
        let response = client.post("/imbue").json(&body).dispatch();
        assert_eq!(response.status(), Status::Ok);

        let result = response.into_json::<ImbueResponse>().unwrap();
        let expected_result = vec![
            DataPointWrapper::new(2.0, 1.0),
            DataPointWrapper::new(4.0, 3.0),
        ];
        assert_eq!(result.dataset, expected_result)
    }
}
