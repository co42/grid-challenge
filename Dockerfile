# Stage 1: Build frontend
FROM node:22-alpine AS frontend
WORKDIR /app/web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/ ./
RUN npm run build

# Stage 2: Build backend
FROM rust:1-alpine AS backend
RUN apk add --no-cache musl-dev
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY migrations/ migrations/
RUN cargo build --release --package grid-challenge-server

# Stage 3: Runtime
FROM alpine:3
RUN apk add --no-cache ca-certificates
WORKDIR /app

COPY --from=backend /app/target/release/grid-challenge-server ./
COPY --from=frontend /app/web/dist ./web/dist

COPY migrations/ migrations/

ENV DATABASE_PATH=data/grid-challenge.db
RUN mkdir -p data

EXPOSE 3000
CMD ["./grid-challenge-server"]
