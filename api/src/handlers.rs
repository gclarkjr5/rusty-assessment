use anyhow::Result;
use rocket::{http::Status, serde::json::Json, State};

use crate::models::{Message, Metrics, DbConnection};

pub type DbConn = State<DbConnection>;

// basic index route
#[get("/")]
pub async fn index() -> &'static str {
    "Hello, world!"
}

// basic route to show JSON response
#[get("/ping")]
pub async fn ping() -> Result<Json<Message>, Status> {
    let result = Message::message("pong".to_string()).await;

    match result {
        Ok(result) => Ok(Json(result)),
        _ => Err(Status::NotFound),
    }
}

// get metrics of the orders
#[get("/orders")]
pub async fn order_metrics(dbconn: &DbConn) -> Result<Json<Metrics>, Status> {

    // returned deserialized JSON metrics
    Ok(
        Json(
            dbconn.publish_metrics().await
        )
    )
}

// view the data the sessionized data
// #[get("/view?<sessionized>&<side>&<nrow>")]
// pub async fn view_data(
//     data: &State<Data>,
//     sessionized: Option<bool>,
//     side: Option<&str>,
//     nrow: Option<u32>,
// ) -> Result<Json<DataView>, Status> {
//     // handle the query params
//     let sessionized = sessionized.unwrap_or(false);
//     let side = side.unwrap_or("top");
//     let nrow = nrow.unwrap_or(5);

//     // process data view
//     let result = data.view_data(sessionized, side, nrow).await;

//     // on success deserialize object into JSON
//     match result {
//         Ok(result) => Ok(Json(result)),
//         _ => Err(Status::NotFound),
//     }
// }

// re-sessionized the data if you want to view other session_lengths without having to re-run
// #[get("/re-sessionize?<session_length>")]
// pub async fn re_sessionize(
//     data: &State<Data>,
//     session_length: Option<u32>,
// ) -> Result<Json<Message>, Status> {
//     // handle query param
//     let session_length = session_length.unwrap_or(30);

//     // process re-sessionization
//     let result = data.re_sessionize(session_length).await;

//     // on success, update state and send out success message of re-sessionization
//     match result {
//         Ok(result) => {
//             // successful re-sessionization should edit state
//             let mut lock = data.sessionizedf.lock().expect("lock shared data");
//             *lock = result;

//             let message = format!(
//                 "Successfully resessionized the data with a session length of {}",
//                 session_length
//             );

//             // return a success message
//             Ok(Json(Message { message }))
//         }
//         _ => Err(Status::NotFound),
//     }
// }


