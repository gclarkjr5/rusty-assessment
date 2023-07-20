FROM rust:1.71.0

WORKDIR /usr/src/api

COPY . .

RUN cargo install --path .

CMD [ "api" ]