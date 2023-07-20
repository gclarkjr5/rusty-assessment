use anyhow::{Result, Ok};
use polars::{lazy::dsl::StrptimeOptions, prelude::*};

use std::io::Cursor;
use std::fs;

use s3::creds::Credentials;
use s3::Bucket;


// object for passing state around to all handlers
pub struct Data {
    pub df: LazyFrame,
}

impl Data {
    // initialize the empty state
    pub async fn init() -> Result<Self> {
        println!("initializing...");
        Ok(Self {
            df: DataFrame::empty().lazy(),
        })
    }

    // stage the data
    pub async fn extract(mut self, url: &str) -> Result<Self> {
        println!("retrieving the data and staging it...");

        // read the body of the response from the requested url
        let body = reqwest::get(url.to_string())
            .await
            .expect("error requesting")
            .bytes()
            .await?;
        
        // create a cursor
        let file = Cursor::new(body);

        // with the cursor, serialize the body of bytes into a Polars DataFrame/LazyFrame
        let df = JsonReader::new(file)
            .with_json_format(JsonFormat::JsonLines)
            .infer_schema_len(Some(20))
            .with_batch_size(100)
            .finish()
            .expect("error finishing dataframe")
            .lazy();
        
        self.df = df;
        Ok(self)
    }

    // initial sessionization for application state
    pub async fn transform(mut self, session_length: u32) -> Result<Self> {
        println!("transforming data by sessionizing it");

        let ld = self.df;

        // sessionize the data
        let df = sessionize(ld, session_length)
            .await
            .expect("error sessionizing");

        self.df = df;
        Ok(self)
    }

    pub async fn load(self, data_path: &str, region: &str, endpoint: &str, access_key: &str, secret_key: &str, bucket: &str) {
        println!("loading data");


        // shouldnt have to do this, should be able to convert a Polars DataFrame into a bytes representation or something
        write_to_parquet(data_path, self.df.clone()).await;
        ///////////////////////////

        write_to_bucket(data_path, region, endpoint, access_key, secret_key, bucket).await;

    }

    // // re-sessionizing the data after initialization
    // pub async fn re_sessionize(&self, session_length: u32) -> Result<LazyFrame> {
    //     // get a copy of the internal sessionized data
    //     let sd = self.lazydf.lock().expect("re-session lock").clone();

    //     // sessionize the data
    //     let df = sessionize(sd, session_length)
    //         .await
    //         .expect("error sessionizing");

    //     // return the resessionized data, not the state object
    //     Ok(df)
    // }
}

async fn sessionize(lazydata: LazyFrame, session_length: u32) -> Result<LazyFrame> {
    println!("sessionizing with a session length of {}", session_length);

    let df = lazydata
        // explode/normalize/unnest the event column which is a struct
        .unnest(["event"])
        // remove null customer ids
        .filter(col("customer-id").is_not_null())
        // cast timestamp column from string to actual timestamp
        .with_column(col("timestamp").str().strptime(
            DataType::Datetime(TimeUnit::Microseconds, None),
            StrptimeOptions {
                format: Some("%Y-%m-%dT%H:%M:%S%.f".into()),
                strict: false,
                exact: true,
                cache: false,
            }  
        ))
        // sort data frame by customer id asc, timestamp asc
        .sort_by_exprs(
            vec![col("customer-id"), col("timestamp")],
            vec![false, false],
            false,
            false,
        )
        // lag/shift the timestamp column by per customer
        .with_column(
            col("timestamp")
                .sort_by(["timestamp"], [false])
                .shift(1)
                .over([col("customer-id")])
                .alias("prev-timestamp"),
        )
        // calculate time difference between timestamp and lagged ts (converted from microsecs to min)
        .with_column(((col("timestamp") - col("prev-timestamp")) / lit(6e7)).alias("time-diff"))
        // fill null time-diffs with 0s
        .with_column(col("time-diff").fill_null(lit(0)))
        // .filter(col("customer-id").eq(609))
        .with_columns([
            when((col("time-diff")).gt(session_length))
                .then(1)
                .otherwise(0)
                .alias("new-session"),
        ])
        .with_columns([
            // accumulate new sessions for each customer in timestamp order
            col("new-session")
                .sort_by(["timestamp"], [false])
                .cumsum(false)
                .over([col("customer-id")])
                .alias("session-number"),
        ])
        // keep only necessary columns
        .select([
            col("customer-id"),
            col("timestamp"),
            col("time-diff"),
            col("new-session"),
            col("session-number"),
            col("type"),
        ]);

    Ok(df)
}

async fn connect_to_bucket(region: &str, endpoint: &str, access_key: &str, secret_key: &str, bucket: &str) -> Result<Bucket> {

    let bucket = Bucket::new(
        bucket,
        s3::Region::Custom {
            region: region.to_owned(),
            endpoint: endpoint.to_owned(),
        },
        get_credentials(Some(access_key), Some(secret_key)).await.expect("error with creds"),
    ).expect("error making bucket")
    .with_path_style();

    Ok(bucket)
}

async fn get_credentials(access_key: Option<&str>, secret_key: Option<&str>) -> Result<Credentials> {
    let creds = s3::creds::Credentials::new(
        access_key, secret_key, None, None, None);

    Ok(creds.expect("error constructing creds"))
}

async fn write_to_parquet(path: &str, df: LazyFrame) {
    let mut file = std::fs::File::create(path).expect("error create file path for parquet");
    ParquetWriter::new(&mut file).finish(&mut df.collect().expect("error to parquet")).expect("another error making parquet writer");
}

async fn write_to_bucket(path: &str, region: &str, endpoint: &str, access_key: &str, secret_key: &str, bucket: &str) {
    let bucket = connect_to_bucket(region, endpoint, access_key, secret_key, bucket).await.expect("error instantiating bucket");

    let bytes = fs::read(path).expect("error reading file path");

    // add file to bucket
    bucket.put_object(path, &bytes).await.expect("error putting object");

    fs::remove_file(path).expect("error removing file");
}