# Copyright (c) 2025 Vigo Browser. All rights reserved.
# Proprietary and confidential. Unauthorized copying prohibited.

FROM golang:1.22-alpine AS builder
WORKDIR /app
COPY go.mod go.sum ./
RUN go mod download
COPY . .
RUN CGO_ENABLED=1 GOOS=linux go build -o vigo-sync-server ./src/main.go

FROM alpine:3.19
RUN apk add --no-cache ca-certificates sqlite-libs
WORKDIR /app
COPY --from=builder /app/vigo-sync-server .
RUN mkdir -p /app/data
EXPOSE 8443

ENTRYPOINT ["/app/vigo-sync-server"]
