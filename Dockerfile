FROM node@sha256:0e0ff40c39bc087845bfb27465a0df4ea419520094bc35842ff83dd8cbe6f9b6 AS frontend
WORKDIR /build
COPY frontend/package*.json ./
RUN npm ci --ignore-scripts
COPY frontend/ ./
RUN npm run build

FROM rust@sha256:a96ea6d18d4062e38f16cfbadd8b4541d622f2527dd0a5eca1fb36d301da4e88 AS backend
RUN apk add --no-cache musl-dev gcc make
WORKDIR /build
COPY backend/ ./
RUN --mount=type=cache,target=/usr/local/cargo/registry --mount=type=cache,target=/build/target,id=okx-musl-target cargo build --release --locked && cp target/release/okx-terminal /okx-terminal && cp target/release/okx-onboarding /okx-onboarding && cp target/release/okx-orders /okx-orders && cp target/release/okx-evaluator /okx-evaluator

FROM backend AS verification
RUN --mount=type=cache,target=/usr/local/cargo/registry --mount=type=cache,target=/build/target,id=okx-musl-target cargo test --release --locked

FROM alpine@sha256:294b683cb724975bec92580e1e685676bd4b50bda910ddb8c51d4cabeaec77e6
RUN apk upgrade --no-cache && apk add --no-cache ca-certificates
RUN mkdir /jobs /archives /onboarding /training && chown 10001:10001 /jobs /archives /onboarding /training
WORKDIR /app
COPY --from=backend /okx-terminal /app/okx-terminal
COPY --from=backend /okx-onboarding /app/okx-onboarding
COPY --from=backend /okx-orders /app/okx-orders
COPY --from=backend /okx-evaluator /app/okx-evaluator
COPY --from=frontend /build/dist /app/public
USER 10001:10001
EXPOSE 8080
HEALTHCHECK --interval=15s --timeout=3s --start-period=30s CMD wget -q -O /dev/null http://127.0.0.1:8080/api/health || exit 1
ENTRYPOINT ["/app/okx-terminal"]
