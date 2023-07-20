use std::io::Result;

use etl::etl::Data;

#[tokio::main]
async fn main() -> Result<()> {
    // load environment variables
    // dotenvy::dotenv().expect("error loading .env vars");
    // run etl
    // get URL & SESSION_LENGTH env vars
    let url = std::env::var("URL").expect("error getting URL");
    let session_length: u32 = std::env::var("SESSION_LENGTH")
        .expect("error getting SESSION_LENGTH")
        .parse::<u32>()
        .expect("error parsing SESSION_LENGTH into u32");
    let data_path = std::env::var("DATA_PATH").expect("error getting DATA_PATH");

    // env vars for accessiing minio
    let region = std::env::var("BUCKET_REGION").expect("error getting BUCKET_REGION");
    let endpoint = std::env::var("BUCKET_ENDPOINT").expect("error getting BUCKET_ENDPOINT");
    let access_key = std::env::var("ACCESS_KEY").expect("error getting ACCESS_KEY");
    let secret_key = std::env::var("SECRET_KEY").expect("error getting SECRET_KEY");
    let bucket = std::env::var("STAGING_BUCKET").expect("error getting bucket name");

    Data::init().await.expect("error initializing data")
        .extract(&url).await.expect("error extracting data")
        .transform(session_length).await.expect("error transforming data")
        .load(&data_path, &region, &endpoint, &access_key, &secret_key, &bucket).await;


    Ok(())
}