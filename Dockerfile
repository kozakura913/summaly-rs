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
COPY examples ./examples
COPY Cargo.toml ./Cargo.toml
RUN --mount=type=cache,target=/var/cache/cargo --mount=type=cache,target=/app/target sh /app/build/build.sh

FROM alpine:latest
COPY --from=0 /app/summaly-rs /
COPY --from=0 /app/healthcheck ./healthcheck
RUN sh -c "./summaly-rs&" && ./healthcheck 5555 http://127.0.0.1:12267/
HEALTHCHECK --interval=30s --timeout=3s CMD ./healthcheck 5555 http://127.0.0.1:12267/ || exit 1
EXPOSE 12267
CMD ["/summaly-rs"]
