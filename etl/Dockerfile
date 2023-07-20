FROM rust:1.71.0

WORKDIR /usr/src/etl

COPY . .

RUN cargo install --path .

CMD [ "etl" ]