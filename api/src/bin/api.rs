use std::io::Result;

fn main() -> Result<()> {
    // boot up web server
    api::router::rocket().expect("error launching api");

    Ok(())
}