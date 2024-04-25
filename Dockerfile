FROM --platform=$BUILDPLATFORM rust:alpine
RUN apk add --no-cache curl musl-dev
ARG BUILDARCH
ARG TARGETARCH
ARG TARGETVARIANT
ENV CARGO_HOME=/var/cache/cargo
COPY build /app/build
WORKDIR /app
RUN sh /app/build/install.sh
COPY src ./src
COPY Cargo.toml ./Cargo.toml
RUN --mount=type=cache,target=/var/cache/cargo --mount=type=cache,target=/app/target sh /app/build/build.sh

FROM scratch
COPY --from=0 /app/summaly-rs /
EXPOSE 12267
CMD ["/summaly-rs"]
