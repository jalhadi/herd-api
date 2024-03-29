FROM rust:1.43

ARG DATABASE_URL_ARG=
ENV DATABASE_URL=$DATABASE_URL_ARG
ARG HMAC_KEY_ARG=
ENV HMAC_KEY=$HMAC_KEY_ARG
ARG API_CIPHER_KEY_ARG=
ENV API_CIPHER_KEY=$API_CIPHER_KEY_ARG

ENV APP_HOME /graph-analytics
RUN mkdir $APP_HOME
WORKDIR $APP_HOME

ADD . $APP_HOME

RUN cargo build --release
RUN cargo install diesel_cli --no-default-features --features postgres
