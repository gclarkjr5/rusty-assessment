FROM ubuntu:latest

WORKDIR /opt

RUN apt-get update && apt-get install -y wget

RUN wget https://github.com/datafuselabs/bendsql/releases/download/v0.4.7/bendsql_0.4.7_arm64.deb

RUN dpkg -i bendsql_0.4.7_arm64.deb && apt --fix-broken install

CMD [ "bendsql" ]