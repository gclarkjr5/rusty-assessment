use anyhow::{Result, Ok};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::thread;
use std::time::Duration;

use databend_driver::{new_connection, Connection, Row};
use tokio_stream::StreamExt;


// object for returning messages
#[derive(Serialize, Deserialize)]
pub struct Message {
    pub message: String,
}

impl Message {
    pub async fn message(message: String) -> Result<Message> {
        Ok(Message { message })
    }
}

#[derive(Clone)]
pub struct DbConnection {
    pub conn: Box<dyn Connection>
}

impl DbConnection {
    pub async fn init(db_user: &str, db_pwd: &str, db_host: &str, db_port: &u32, db: &str) -> Result<Self> {
        let dsn = format!("databend://{db_user}:{db_pwd}@{db_host}:{db_port}/{db}?sslmode=disable");

        println!("establishing connection to {}", dsn);
        let conn = new_connection(&dsn).expect("error making connection");

        Ok(DbConnection {conn})
    }

    pub async fn prepare_db(&self, endpoint: &str, access_key: &str, secret_key: &str, bucket: &str) -> &Self {
        let conn = &self.conn;
    
        // create the database
        let sql_db_create = "CREATE DATABASE IF NOT EXISTS webshop;";
        println!("preparing database");
        conn.exec(sql_db_create).await.expect("error creating database");
    
        // create the table
        let sql_table_create = "
            CREATE TABLE IF NOT EXISTS webshop.events (
                customer_id int,
                timestamp timestamp,
                time_diff int,
                new_session int,
                session_number int,
                type varchar
            );
        ";

        println!("preparing table");
        conn.exec(sql_table_create).await.expect("error creating table");

        // create the stage for the staged data
        let create_stage = format!("
        CREATE STAGE IF NOT EXISTS sessionized
            URL='s3://{bucket}/'
            CONNECTION = (
                ENDPOINT_URL = '{endpoint}'
                ACCESS_KEY_ID = '{access_key}'
                SECRET_ACCESS_KEY = '{secret_key}'
            );
        ");
    
        println!("preparing stage");
        conn.exec(&create_stage).await.expect("error creating stage");

        self
    
    }

    pub async fn copy_stage_to_table(&self) {
        let conn = &self.conn;

        // first lets see if data is in the stage
        let select_stage_sql = "select * from @sessionized limit 10;";
        println!("testing if data is sessionized yet");

        while let Err(e) = conn.exec(select_stage_sql).await {
            println!("Error: {}", e);
            println!("sleeping for 5 seconds");

            thread::sleep(Duration::from_secs(5));
        }
        
        // copy data from the stage to the table
        let copy_stage_to_table = "
        COPY INTO webshop.events
        FROM (
            SELECT *
            FROM @sessionized
        );
        ";
    
        println!("copying data from stage into table");
        conn.exec(copy_stage_to_table).await.expect("error copy data from stage into table");
    }

    // pub async fn 

    pub async fn publish_metrics(&self) -> Metrics {
        let conn = &self.conn;

        // median sessions
        let median_visits_before_order_sql = "
            -- identify events where an order was placed
            with placed_order_events as (
                select
                    *,
                    case
                        when type = 'placed_order' then 1
                        else 0
                    end as placed_order
                from webshop.events
            ),

            -- accumulate the orders
            order_numbers as (
                select
                    *,
                    sum(placed_order) over(partition by customer_id order by timestamp) as order_number
                from placed_order_events
            ),

            -- get the max session that occurred for every placed order
            max_session_per_order as (
                select
                    *,
                    max(session_number) over(partition by customer_id, order_number) as max_session_for_order
                from order_numbers
            ),

            -- grab the distinct sets
            distinct_sessions_orders as (
                select distinct
                    customer_id, order_number, max_session_for_order
                from max_session_per_order
                order by order_number
            ),

            -- lag the max session of each placed order to see what the previous one was
            lag_max_session_of_order as (
                select
                    *,
                    coalesce(lag(max_session_for_order) over(partition by customer_id order by order_number), 0) as lagged_max_session_order
                from distinct_sessions_orders
            ),

            -- diff the current - the lag to see how many session occurred between each placed order
            session_diffs as (
                select 
                    *,
                    max_session_for_order - lagged_max_session_order as session_diff
                from lag_max_session_of_order
            ),

            -- median the result
            final as (
                select
                    median(session_diff)
                from session_diffs
            )

            select
                *
            from final;

            
        ";

        let median_session_duration_minutes_before_order_sql = "
            -- identify events where an order was placed
            with placed_order_events as (
                select
                    *,
                    case
                        when type = 'placed_order' then 1
                        else 0
                    end as placed_order
                from webshop.events
            ),

            -- accumulate the orders
            order_numbers as (
                select
                    *,
                    sum(placed_order) over(partition by customer_id order by timestamp) as order_number
                from placed_order_events
            ),

            -- get all events before the first order which also bring all sessions before first order
            events_before_first_order as (
                select
                    *
                from order_numbers
                where order_number = 0
            ),

            -- get duration of each session
            session_metrics as (
                select
                    customer_id,
                    timestamp,
                    session_number,
                    min(timestamp) over(partition by customer_id, session_number) as min_session_event,
                    max(timestamp) over(partition by customer_id, session_number) as max_session_event
                from events_before_first_order
            ),

            -- calculate duration
            session_duration as (
                select distinct
                    customer_id,
                    session_number,
                    ((max_session_event - min_session_event)/1000000)/60 as session_duration
                from session_metrics
            ),

            -- calculate median
            final as (
                select
                    median(session_duration)
                from session_duration
            )

            select *
            from final;
        ";

        
        
        let mv = conn.query_iter(median_visits_before_order_sql).await.unwrap().next().await.unwrap().unwrap();
        let md = conn.query_iter(median_session_duration_minutes_before_order_sql).await.unwrap().next().await.unwrap().unwrap();

        
        Metrics {
            median_visits_before_order: extract_values(mv).await.unwrap(),
            median_session_duration_minutes_before_order: extract_values(md).await.unwrap()
        }

    }

}

// object for viewing metrics
#[derive(Debug, Serialize, Deserialize)]
pub struct Metrics {
    pub median_visits_before_order: f64,
    pub median_session_duration_minutes_before_order: f64,
}

async fn extract_values(row: Row) -> Result<f64> {
    let value = row.values()[0].clone();
    let conversion: Result<f64, _> = value.try_into();
    let res = conversion.expect("error converting value to f64");

    Ok(res)
}

