FROM rust:1.43

ENV APP_HOME /graph-analytics
RUN mkdir $APP_HOME
WORKDIR $APP_HOME

ADD . $APP_HOME

RUN cargo build --release
