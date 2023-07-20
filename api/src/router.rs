use color_eyre::eyre::{Error, Result};

use crate::handlers::*;
use crate::models::DbConnection;
// use etl::etl::Data;

#[rocket::main]
pub async fn rocket() -> Result<(), Error> {
    // pretty error handling
    color_eyre::install()?;

    // load environment variables
    // dotenvy::dotenv().expect("error loading .env vars");

    // get URL & SESSION_LENGTH env vars
    // let url = std::env::var("URL").expect("error getting URL");
    // let session_length: u32 = std::env::var("SESSION_LENGTH")
    //     .expect("error getting SESSION_LENGTH")
    //     .parse::<u32>()
    //     .expect("error parsing SESSION_LENGTH into u32");
    // let data_path = std::env::var("DATA_PATH").expect("error getting DATA_PATH");

    // env vars for accessiing minio
    // let region = std::env::var("BUCKET_REGION").expect("error getting BUCKET_REGION");
    let endpoint = std::env::var("BUCKET_ENDPOINT").expect("error getting BUCKET_ENDPOINT");
    let access_key = std::env::var("ACCESS_KEY").expect("error getting ACCESS_KEY");
    let secret_key = std::env::var("SECRET_KEY").expect("error getting SECRET_KEY");
    let bucket = std::env::var("STAGING_BUCKET").expect("error getting bucket name");

    // env vars for connecting to databend
    let db_user = std::env::var("DATABEND_USER").expect("error getting DATABEND_USER");
    let db_pwd = std::env::var("DATABEND_PWD").expect("error getting DATABEND_PWD");
    let db_host = std::env::var("DATABEND_HOST").expect("error getting DATABEND_HOST");
    let db_port = std::env::var("DATABEND_PORT")
        .expect("error getting DATABEND_PORT")
        .parse::<u32>()
        .expect("error parsing DATABEND_PORT into u32");
    let db = std::env::var("DATABEND_DB").expect("error getting DATABEND_DB");

    // create the initial state
    // initialize the Data
    // extract the data: request it from the URL, sessionize, then store it

    // Data::init().await.expect("error initializing data")
    //     .extract(&url).await.expect("error extracting data")
    //     .transform(session_length).await.expect("error transforming data")
    //     .load(&data_path, &region, &endpoint, &access_key, &secret_key).await;


    let state = DbConnection::init(&db_user, &db_pwd, &db_host, &db_port, &db).await.expect("error connecting to db");

    state
        .prepare_db(&endpoint, &access_key, &secret_key, &bucket).await
        .copy_stage_to_table().await;

    // setup router with several mounts and the handlers that belong to each mount
    // pass the state around to the handlers
    let _ = rocket::build()
        .manage(state)
        .mount("/", routes![index, ping])
        // .mount("/data", routes![view_data, re_sessionize,])
        .mount("/metrics", routes![order_metrics,])
        .launch()
        .await?;

    Ok(())
}
